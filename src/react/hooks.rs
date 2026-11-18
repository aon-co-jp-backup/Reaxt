//! コンポーネントモデル第一段: 関数コンポーネント + `use_state`フック。
//!
//! Reactの「フックはコンポーネントごとに呼び出し順序が一定」という
//! ルールと同じ制約を踏襲する——`use_state`はコンポーネントの
//! render関数の中で、分岐やループの外側で呼ぶ前提(呼び出し順序が
//! そのままフックの識別子になる、Reactの実装と同じ割り切り)。
//!
//! 状態はコンポーネントインスタンスの識別子(`ComponentId`、呼び出し側が
//! 割り当てる——ツリー上の位置や明示的な`key`から決める想定)に紐づけて
//! `HOOK_STORE`(スレッドローカル)に永続化し、再レンダー(`render`の
//! 再実行)をまたいで保持する。

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;

/// コンポーネントインスタンスの識別子。呼び出し側(アプリのレンダー
/// ループ)がツリー上の位置や`key`から決定し、同じインスタンスに対して
/// 再レンダーのたびに同じ値を渡す責務を持つ。
pub type ComponentId = u64;

struct HookFrame {
    states: Vec<Box<dyn Any>>,
    cursor: usize,
}

impl HookFrame {
    fn new() -> Self {
        Self { states: Vec::new(), cursor: 0 }
    }
}

thread_local! {
    static HOOK_STORE: RefCell<HashMap<ComponentId, HookFrame>> = RefCell::new(HashMap::new());
    static ACTIVE_STACK: RefCell<Vec<ComponentId>> = RefCell::new(Vec::new());
    static DIRTY: RefCell<std::collections::HashSet<ComponentId>> = RefCell::new(std::collections::HashSet::new());
}

/// コンポーネントのrender関数を呼び出す前に、そのインスタンスの
/// フックカーソルをリセットする。render関数呼び出しの直前に必ず対で
/// 呼ぶこと(`run_component`を使えば自動で対になる)。
fn begin_render(id: ComponentId) {
    HOOK_STORE.with(|store| {
        store.borrow_mut().entry(id).or_insert_with(HookFrame::new).cursor = 0;
    });
    ACTIVE_STACK.with(|stack| stack.borrow_mut().push(id));
}

fn end_render(_id: ComponentId) {
    ACTIVE_STACK.with(|stack| {
        stack.borrow_mut().pop();
    });
}

fn current_id() -> ComponentId {
    ACTIVE_STACK.with(|stack| {
        *stack
            .borrow()
            .last()
            .expect("use_state must be called during a component's render (no active component on the stack)")
    })
}

/// `render_fn`を、フックの状態を`id`に紐づけた状態で1回実行する。
/// `render_fn`の中で呼ばれる`use_state`は、この`id`のフックストアを見る。
pub fn run_component<T>(id: ComponentId, render_fn: impl FnOnce() -> T) -> T {
    begin_render(id);
    let result = render_fn();
    end_render(id);
    // 一番外側の描画が終わった時点で、溜まった副作用(use_effect)を流す
    if ACTIVE_STACK.with(|s| s.borrow().is_empty()) {
        flush_effects();
    }
    result
}

/// 直前の`run_component`呼び出し以降、このコンポーネントインスタンスが
/// 再レンダーを要求したか(`StateSetter`が呼ばれたか)を確認し、
/// フラグを消費する(一度読んだら消える——次の変化まで再度falseになる)。
pub fn take_dirty(id: ComponentId) -> bool {
    DIRTY.with(|dirty| dirty.borrow_mut().remove(&id))
}

/// ツリー全体を通じて、いずれかのコンポーネントインスタンスが
/// 再レンダーを要求しているか(消費はしない、`App::tick`が「今回は
/// 再レンダーが必要か」を判定するための問い合わせ専用)。
pub fn any_dirty() -> bool {
    DIRTY.with(|dirty| !dirty.borrow().is_empty())
}

/// dirtyフラグを全インスタンスまとめて消費する。ルートから丸ごと
/// 再レンダーする「アプリループ」(`App::tick`)は、個々のインスタンス
/// ごとに`take_dirty`する代わりに、再レンダー開始時にまとめて消費する
/// ——丸ごと再レンダーする以上、個別のdirtyフラグを取りこぼす心配は無い
/// (どの`ComponentId`がdirtyだったかを問わず、次回の変化まで再度
/// `any_dirty`がfalseを返すようにするのが目的)。
pub fn clear_all_dirty() {
    DIRTY.with(|dirty| dirty.borrow_mut().clear());
}

/// 状態を更新するためのハンドル。`set`を呼ぶと即座にストアへ反映され、
/// 対応する`ComponentId`が`take_dirty`で観測可能になる
/// (Reactのように非同期にバッチングはしない、素朴な同期反映)。
pub struct StateSetter<T> {
    id: ComponentId,
    index: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<T: 'static> StateSetter<T> {
    pub fn set(&self, value: T) {
        HOOK_STORE.with(|store| {
            let mut store = store.borrow_mut();
            let frame = store.get_mut(&self.id).expect("hook frame must exist before its setter is used");
            frame.states[self.index] = Box::new(value);
        });
        DIRTY.with(|dirty| {
            dirty.borrow_mut().insert(self.id);
        });
    }

    /// 現在値を受け取って新しい値を返す関数で更新する(Reactの
    /// `setState(prev => next)`相当)。
    pub fn update(&self, f: impl FnOnce(&T) -> T) {
        let next = HOOK_STORE.with(|store| {
            let store = store.borrow();
            let frame = store.get(&self.id).expect("hook frame must exist before its setter is used");
            let current = frame.states[self.index].downcast_ref::<T>().expect("use_state type mismatch");
            f(current)
        });
        self.set(next);
    }
}

impl<T> Clone for StateSetter<T> {
    fn clone(&self) -> Self {
        Self { id: self.id, index: self.index, _marker: std::marker::PhantomData }
    }
}

/// `use_state`フック。現在の値のクローンと、更新用の`StateSetter`を返す。
/// `init`は初回(このインスタンス・このフック位置で初めて呼ばれた時)
/// のみ評価される。
pub fn use_state<T: Clone + 'static>(init: impl FnOnce() -> T) -> (T, StateSetter<T>) {
    let id = current_id();
    let index = HOOK_STORE.with(|store| {
        let mut store = store.borrow_mut();
        let frame = store.get_mut(&id).expect("use_state called outside of run_component");
        let index = frame.cursor;
        frame.cursor += 1;
        if index == frame.states.len() {
            frame.states.push(Box::new(init()));
        }
        index
    });
    let value = HOOK_STORE.with(|store| {
        let store = store.borrow();
        let frame = store.get(&id).unwrap();
        frame.states[index].downcast_ref::<T>().expect("use_state type mismatch (called with a different type than the previous render)").clone()
    });
    (value, StateSetter { id, index, _marker: std::marker::PhantomData })
}

/// テスト・ツール専用: 指定インスタンスのフック状態を破棄する
/// (コンポーネントがツリーから外れた際の後始末に相当、次段階で
/// アプリのアンマウント処理から呼ぶ想定)。
pub fn drop_instance(id: ComponentId) {
    // use_effect の後始末を先に呼ぶ(ストアを借りていない状態で)
    let mine: Vec<Cleanup> = CLEANUPS.with(|c| {
        let mut c = c.borrow_mut();
        let keys: Vec<(ComponentId, usize)> = c.keys().filter(|(i, _)| *i == id).cloned().collect();
        keys.into_iter().filter_map(|k| c.remove(&k)).collect()
    });
    for f in mine {
        f();
    }
    HOOK_STORE.with(|store| {
        store.borrow_mut().remove(&id);
    });
    DIRTY.with(|dirty| {
        dirty.borrow_mut().remove(&id);
    });
}


// ───── 追加フック: use_ref / use_memo / use_reducer / use_effect / コンテキスト ─────

/// 呼び出し順で1つのスロットを確保し、その位置を返す(初回だけ`init`で作る)。
fn slot<T: 'static>(id: ComponentId, init: impl FnOnce() -> T) -> usize {
    HOOK_STORE.with(|store| {
        let mut store = store.borrow_mut();
        let frame = store.get_mut(&id).expect("hook called outside of run_component");
        let index = frame.cursor;
        frame.cursor += 1;
        if index == frame.states.len() {
            frame.states.push(Box::new(init()));
        }
        index
    })
}

fn with_slot<T: 'static, R>(id: ComponentId, index: usize, f: impl FnOnce(&mut T) -> R) -> R {
    HOOK_STORE.with(|store| {
        let mut store = store.borrow_mut();
        let frame = store.get_mut(&id).expect("hook frame must exist");
        f(frame.states[index].downcast_mut::<T>().expect("hook type mismatch (different type than the previous render)"))
    })
}

/// 再レンダーをまたいで保たれる可変の入れ物(Reactの`useRef`)。書き換えても再レンダーは起きない。
pub type Ref<T> = std::rc::Rc<RefCell<T>>;

pub fn use_ref<T: 'static>(init: impl FnOnce() -> T) -> Ref<T> {
    let id = current_id();
    let i = slot(id, || std::rc::Rc::new(RefCell::new(init())));
    with_slot::<Ref<T>, _>(id, i, |r| r.clone())
}

/// 依存`deps`が変わった時だけ`f`を再計算して値を返す(Reactの`useMemo`)。`use_callback`もこれで足りる。
pub fn use_memo<D: PartialEq + Clone + 'static, T: Clone + 'static>(deps: D, f: impl FnOnce() -> T) -> T {
    let id = current_id();
    let i = slot::<Option<(D, T)>>(id, || None);
    with_slot::<Option<(D, T)>, _>(id, i, |s| {
        if let Some((d, v)) = s {
            if *d == deps {
                return v.clone();
            }
        }
        let v = f();
        *s = Some((deps, v.clone()));
        v
    })
}

/// `use_reducer`の更新口。`dispatch(action)`で`reducer(&現在値, action)`の結果に置き換える。
pub struct Dispatch<S, A> {
    id: ComponentId,
    index: usize,
    reducer: fn(&S, A) -> S,
}

impl<S: 'static, A> Dispatch<S, A> {
    pub fn dispatch(&self, action: A) {
        let r = self.reducer;
        with_slot::<S, _>(self.id, self.index, |s| *s = r(s, action));
        DIRTY.with(|d| {
            d.borrow_mut().insert(self.id);
        });
    }
}

impl<S, A> Clone for Dispatch<S, A> {
    fn clone(&self) -> Self {
        Self { id: self.id, index: self.index, reducer: self.reducer }
    }
}

/// Reactの`useReducer`。
pub fn use_reducer<S: Clone + 'static, A>(reducer: fn(&S, A) -> S, init: impl FnOnce() -> S) -> (S, Dispatch<S, A>) {
    let id = current_id();
    let i = slot(id, init);
    let v = with_slot::<S, _>(id, i, |s| s.clone());
    (v, Dispatch { id, index: i, reducer })
}

/// 副作用の後始末関数(次の実行の前、またはコンポーネントの破棄時に呼ぶ)。
pub type Cleanup = Box<dyn FnOnce()>;

struct EffectSlot<D> {
    deps: Option<D>,
}

thread_local! {
    /// 描画後に実行する副作用の待ち行列(`run_component`が描画の直後に流す)。
    static PENDING_EFFECTS: RefCell<Vec<Box<dyn FnOnce()>>> = RefCell::new(Vec::new());
    /// 破棄時に呼ぶ後始末(インスタンスごと)。
    static CLEANUPS: RefCell<HashMap<(ComponentId, usize), Cleanup>> = RefCell::new(HashMap::new());
}

/// Reactの`useEffect`。描画の後に`f`を実行し、返した後始末は次回の実行前・破棄時に呼ぶ。
/// `deps`が前回と同じなら実行しない(毎回実行したい時は、毎回変わる値を渡す)。
pub fn use_effect<D: PartialEq + Clone + 'static>(deps: D, f: impl FnOnce() -> Option<Cleanup> + 'static) {
    let id = current_id();
    let i = slot::<EffectSlot<D>>(id, || EffectSlot { deps: None });
    let changed = with_slot::<EffectSlot<D>, _>(id, i, |s| s.deps.as_ref() != Some(&deps));
    if !changed {
        return;
    }
    with_slot::<EffectSlot<D>, _>(id, i, |s| s.deps = Some(deps));
    PENDING_EFFECTS.with(|q| {
        q.borrow_mut().push(Box::new(move || {
            // 前回の後始末 → 今回の実行(どちらもストアを借りない状態で呼ぶ)
            let prev = CLEANUPS.with(|c| c.borrow_mut().remove(&(id, i)));
            if let Some(c) = prev {
                c();
            }
            let next = f();
            if let Some(n) = next {
                CLEANUPS.with(|c| c.borrow_mut().insert((id, i), n));
            }
        }));
    });
}

fn flush_effects() {
    loop {
        let batch: Vec<Box<dyn FnOnce()>> = PENDING_EFFECTS.with(|q| std::mem::take(&mut *q.borrow_mut()));
        if batch.is_empty() {
            break;
        }
        for e in batch {
            e();
        }
    }
}

// ── コンテキスト(Reactの`createContext`/`Provider`/`useContext`) ──

thread_local! {
    static CONTEXTS: RefCell<Vec<(std::any::TypeId, Box<dyn Any>)>> = RefCell::new(Vec::new());
}

/// `render`を実行する間だけ、子孫が`use_context::<T>()`で`value`を読めるようにする(Provider)。
/// 内側の`provide_context`は外側より優先される。
pub fn provide_context<T: Clone + 'static, R>(value: T, render: impl FnOnce() -> R) -> R {
    CONTEXTS.with(|c| c.borrow_mut().push((std::any::TypeId::of::<T>(), Box::new(value))));
    let r = render();
    CONTEXTS.with(|c| {
        c.borrow_mut().pop();
    });
    r
}

/// 一番近いProviderの値を返す(無ければ`None`)。型`T`がそのままコンテキストの識別子になる。
pub fn use_context<T: Clone + 'static>() -> Option<T> {
    CONTEXTS.with(|c| {
        c.borrow().iter().rev().find(|(t, _)| *t == std::any::TypeId::of::<T>()).and_then(|(_, v)| v.downcast_ref::<T>().cloned())
    })
}


/// Reactの`useId`。コンポーネントとフック位置ごとに安定した一意の文字列(`label`の`for`などに使う)。
pub fn use_id() -> String {
    let id = current_id();
    let i = slot::<()>(id, || ());
    format!("rx-{id}-{i}")
}

/// Reactの`useLayoutEffect`。描画が同期でレイアウト待ちが無いため、`use_effect`と同じ(描画直後に実行)。
pub fn use_layout_effect<D: PartialEq + Clone + 'static>(deps: D, f: impl FnOnce() -> Option<Cleanup> + 'static) {
    use_effect(deps, f)
}

/// Reactの`useDeferredValue`。更新を遅らせる並行描画が無いため、値をそのまま返す。
pub fn use_deferred_value<T>(v: T) -> T {
    v
}

/// Reactの`useTransition`相当。並行描画が無いので `is_pending` は常に false、`start` は即実行する。
pub fn use_transition() -> (bool, fn(&dyn Fn())) {
    fn start(f: &dyn Fn()) {
        f()
    }
    (false, start)
}

/// Reactの`memo`。`props`が前回と同じなら描画関数を呼ばず前回のVNodeを返す。
pub fn memo<P: PartialEq + Clone + 'static>(props: P, render: impl FnOnce(&P) -> super::vnode::VNode) -> super::vnode::VNode {
    let p = props.clone();
    use_memo(props, move || render(&p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn use_ref_persists_and_does_not_mark_dirty() {
        let id = 20;
        let r = run_component(id, || use_ref(|| 1i32));
        *r.borrow_mut() = 7;
        let r2 = run_component(id, || use_ref(|| 1i32));
        assert_eq!(*r2.borrow(), 7);
        assert!(!take_dirty(id));
        drop_instance(id);
    }

    #[test]
    fn use_memo_recomputes_only_when_deps_change() {
        let id = 21;
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        let go = |d: i32| {
            let c = calls.clone();
            run_component(id, || {
                use_memo(d, move || {
                    c.set(c.get() + 1);
                    d * 2
                })
            })
        };
        assert_eq!(go(1), 2);
        assert_eq!(go(1), 2);
        assert_eq!(calls.get(), 1);
        assert_eq!(go(5), 10);
        assert_eq!(calls.get(), 2);
        drop_instance(id);
    }

    #[test]
    fn use_reducer_applies_actions_and_marks_dirty() {
        fn red(s: &i32, a: i32) -> i32 {
            s + a
        }
        let id = 22;
        let (_, d) = run_component(id, || use_reducer(red, || 10));
        d.dispatch(5);
        d.dispatch(1);
        assert!(take_dirty(id));
        let (v, _) = run_component(id, || use_reducer(red, || 10));
        assert_eq!(v, 16);
        drop_instance(id);
    }

    #[test]
    fn use_effect_runs_after_render_on_deps_change_and_cleans_up() {
        let id = 23;
        let log = std::rc::Rc::new(RefCell::new(Vec::<String>::new()));
        let go = |d: i32| {
            let l = log.clone();
            run_component(id, || {
                use_effect(d, move || {
                    l.borrow_mut().push(format!("run{d}"));
                    let l2 = l.clone();
                    Some(Box::new(move || l2.borrow_mut().push(format!("clean{d}"))) as Cleanup)
                });
            })
        };
        go(1);
        go(1); // 依存が同じ→実行しない
        go(2); // 前回の後始末→今回の実行
        assert_eq!(*log.borrow(), vec!["run1", "clean1", "run2"]);
        drop_instance(id); // 破棄時の後始末
        assert_eq!(*log.borrow(), vec!["run1", "clean1", "run2", "clean2"]);
    }

    #[test]
    fn context_nearest_provider_wins_and_none_outside() {
        assert_eq!(use_context::<String>(), None);
        let inner = provide_context("outer".to_string(), || {
            let a = use_context::<String>();
            let b = provide_context("inner".to_string(), use_context::<String>);
            (a, b)
        });
        assert_eq!(inner, (Some("outer".to_string()), Some("inner".to_string())));
        assert_eq!(use_context::<String>(), None);
    }

    #[test]
    fn use_state_returns_initial_value_on_first_render() {
        let id = 1;
        let value = run_component(id, || {
            let (count, _set) = use_state(|| 0i32);
            count
        });
        assert_eq!(value, 0);
        drop_instance(id);
    }

    #[test]
    fn setter_updates_value_seen_on_next_render() {
        let id = 2;
        let setter = run_component(id, || {
            let (_count, set) = use_state(|| 0i32);
            set
        });
        setter.set(5);
        let value = run_component(id, || {
            let (count, _set) = use_state(|| 0i32);
            count
        });
        assert_eq!(value, 5);
        drop_instance(id);
    }

    #[test]
    fn update_reads_previous_value() {
        let id = 3;
        let setter = run_component(id, || {
            let (_count, set) = use_state(|| 10i32);
            set
        });
        setter.update(|prev| prev + 1);
        let value = run_component(id, || {
            let (count, _set) = use_state(|| 10i32);
            count
        });
        assert_eq!(value, 11);
        drop_instance(id);
    }

    #[test]
    fn setter_marks_component_dirty_and_take_dirty_consumes_it() {
        let id = 4;
        let setter = run_component(id, || {
            let (_count, set) = use_state(|| 0i32);
            set
        });
        assert!(!take_dirty(id));
        setter.set(1);
        assert!(take_dirty(id));
        assert!(!take_dirty(id), "take_dirty must consume the flag");
        drop_instance(id);
    }

    #[test]
    fn multiple_hooks_in_one_component_track_independent_slots() {
        let id = 5;
        let (name_setter, age_setter) = run_component(id, || {
            let (_name, set_name) = use_state(|| "alice".to_string());
            let (_age, set_age) = use_state(|| 30i32);
            (set_name, set_age)
        });
        age_setter.set(31);
        let (name, age) = run_component(id, || {
            let (name, _) = use_state(|| "alice".to_string());
            let (age, _) = use_state(|| 30i32);
            (name, age)
        });
        assert_eq!(name, "alice");
        assert_eq!(age, 31);
        name_setter.set("bob".to_string());
        drop_instance(id);
    }

    #[test]
    fn any_dirty_reflects_whether_any_instance_is_dirty_and_clear_all_dirty_resets_it() {
        let id_a = 8;
        let id_b = 9;
        let setter_a = run_component(id_a, || {
            let (_count, set) = use_state(|| 0i32);
            set
        });
        run_component(id_b, || {
            let (_count, _set) = use_state(|| 0i32);
        });
        assert!(!any_dirty());
        setter_a.set(1);
        assert!(any_dirty());
        clear_all_dirty();
        assert!(!any_dirty());
        assert!(!take_dirty(id_a), "clear_all_dirty must also clear the per-instance flag");
        drop_instance(id_a);
        drop_instance(id_b);
    }

    #[test]
    fn different_component_instances_have_independent_state() {
        let id_a = 6;
        let id_b = 7;
        let setter_a = run_component(id_a, || {
            let (_count, set) = use_state(|| 100i32);
            set
        });
        setter_a.set(200);
        let value_b = run_component(id_b, || {
            let (count, _set) = use_state(|| 100i32);
            count
        });
        assert_eq!(value_b, 100, "instance b must not see instance a's update");
        drop_instance(id_a);
        drop_instance(id_b);
    }
}

#[cfg(test)]
mod more_tests {
    use super::*;
    use crate::react::vnode::VNode;

    #[test]
    fn use_id_is_stable_and_unique_per_hook() {
        let id = 30;
        let a = run_component(id, || (use_id(), use_id()));
        let b = run_component(id, || (use_id(), use_id()));
        assert_eq!(a, b);
        assert_ne!(a.0, a.1);
        drop_instance(id);
    }

    #[test]
    fn memo_skips_render_when_props_unchanged() {
        let id = 31;
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        let go = |p: i32| {
            let c = calls.clone();
            run_component(id, || {
                memo(p, move |p| {
                    c.set(c.get() + 1);
                    VNode::text(format!("{p}"))
                })
            })
        };
        assert_eq!(go(1), VNode::text("1"));
        assert_eq!(go(1), VNode::text("1"));
        assert_eq!(calls.get(), 1);
        assert_eq!(go(2), VNode::text("2"));
        assert_eq!(calls.get(), 2);
        drop_instance(id);
    }
}
