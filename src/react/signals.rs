//! Solid型のシグナル(細粒度リアクティブ): `create_signal` / `create_effect` / `create_memo`。
//! 仮想DOMの再描画とは別の流儀で、値が変わった時に、その値を読んでいた副作用だけを再実行する。
//! 単一スレッド前提(wasm/ブラウザと同じ)。まとめ更新(batch)は未実装。

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

struct Node {
    f: RefCell<Box<dyn FnMut()>>,
    running: Cell<bool>,
    /// 実行のたびに増える。古い購読(前回の実行で読んだ値)は無視するための世代。
    epoch: Cell<u64>,
}

thread_local! {
    static OBSERVER: RefCell<Vec<Rc<Node>>> = RefCell::new(Vec::new());
    /// `batch` の入れ子の深さと、その間に溜まった再実行待ち(重複なし)。
    static BATCH: RefCell<(u32, Vec<Rc<Node>>)> = RefCell::new((0, Vec::new()));
}

/// 複数の `set` をまとめ、`f` が終わった時点で、影響を受けた副作用を1回ずつだけ再実行する。入れ子可。
pub fn batch<R>(f: impl FnOnce() -> R) -> R {
    BATCH.with(|b| b.borrow_mut().0 += 1);
    let r = f();
    let pending = BATCH.with(|b| {
        let mut b = b.borrow_mut();
        b.0 -= 1;
        if b.0 == 0 {
            std::mem::take(&mut b.1)
        } else {
            Vec::new()
        }
    });
    for n in pending {
        run(&n);
    }
    r
}

fn run(node: &Rc<Node>) {
    if node.running.get() {
        return; // 自分自身の更新による再入は無視(無限ループ防止)
    }
    node.running.set(true);
    node.epoch.set(node.epoch.get() + 1);
    OBSERVER.with(|o| o.borrow_mut().push(node.clone()));
    (node.f.borrow_mut())();
    OBSERVER.with(|o| {
        o.borrow_mut().pop();
    });
    node.running.set(false);
}

struct Sub {
    node: Weak<Node>,
    epoch: u64,
}

struct Inner<T> {
    value: T,
    subs: Vec<Sub>,
}

/// 読み書きできる値。`get`した副作用/メモは、`set`で再実行される。
pub struct Signal<T>(Rc<RefCell<Inner<T>>>);

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Signal(self.0.clone())
    }
}

pub fn create_signal<T: Clone + 'static>(init: T) -> Signal<T> {
    Signal(Rc::new(RefCell::new(Inner { value: init, subs: Vec::new() })))
}

impl<T: Clone + 'static> Signal<T> {
    /// 値を読む。副作用/メモの実行中なら、その依存として覚える。
    pub fn get(&self) -> T {
        if let Some(n) = OBSERVER.with(|o| o.borrow().last().cloned()) {
            let mut inner = self.0.borrow_mut();
            let epoch = n.epoch.get();
            let known = inner.subs.iter().any(|s| s.epoch == epoch && s.node.upgrade().is_some_and(|x| Rc::ptr_eq(&x, &n)));
            if !known {
                inner.subs.push(Sub { node: Rc::downgrade(&n), epoch });
            }
        }
        self.0.borrow().value.clone()
    }

    /// 値を読むが依存にはしない。
    pub fn peek(&self) -> T {
        self.0.borrow().value.clone()
    }

    pub fn set(&self, v: T) {
        let targets: Vec<Rc<Node>> = {
            let mut inner = self.0.borrow_mut();
            inner.value = v;
            inner.subs.retain(|s| s.node.upgrade().is_some_and(|n| n.epoch.get() == s.epoch));
            inner.subs.iter().filter_map(|s| s.node.upgrade()).collect()
        };
        for n in targets {
            let queued = BATCH.with(|b| {
                let mut b = b.borrow_mut();
                if b.0 == 0 {
                    return false;
                }
                if !b.1.iter().any(|x| Rc::ptr_eq(x, &n)) {
                    b.1.push(n.clone());
                }
                true
            });
            if !queued {
                run(&n);
            }
        }
    }

    pub fn update(&self, f: impl FnOnce(&T) -> T) {
        let next = f(&self.0.borrow().value);
        self.set(next);
    }
}

/// 副作用。すぐ1回実行し、読んだシグナルが変わるたびに再実行する。返す `Effect` を保持している間だけ有効。
pub struct Effect(Rc<Node>);

impl Effect {
    /// 以後は再実行しない。
    pub fn stop(&self) {
        self.0.epoch.set(u64::MAX); // どの購読の世代とも一致しない
        *self.0.f.borrow_mut() = Box::new(|| {});
    }
}

pub fn create_effect(f: impl FnMut() + 'static) -> Effect {
    let node = Rc::new(Node { f: RefCell::new(Box::new(f)), running: Cell::new(false), epoch: Cell::new(0) });
    run(&node);
    Effect(node)
}

/// 派生値。依存が変わった時だけ再計算し、結果が変わらなければ購読者を起こさない。
pub struct Memo<T> {
    sig: Signal<T>,
    _effect: Rc<Effect>,
}

impl<T> Clone for Memo<T> {
    fn clone(&self) -> Self {
        Memo { sig: self.sig.clone(), _effect: self._effect.clone() }
    }
}

impl<T: Clone + PartialEq + 'static> Memo<T> {
    pub fn get(&self) -> T {
        self.sig.get()
    }
}

pub fn create_memo<T: Clone + PartialEq + 'static>(mut f: impl FnMut() -> T + 'static) -> Memo<T> {
    let slot: Rc<RefCell<Option<Signal<T>>>> = Rc::new(RefCell::new(None));
    let s2 = slot.clone();
    let effect = create_effect(move || {
        let v = f();
        let existing = s2.borrow().clone();
        match existing {
            None => *s2.borrow_mut() = Some(create_signal(v)),
            Some(sig) => {
                if sig.peek() != v {
                    sig.set(v);
                }
            }
        }
    });
    let sig = slot.borrow().clone().expect("memoは作成時に1回計算される");
    Memo { sig, _effect: Rc::new(effect) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_reruns_only_for_signals_it_read() {
        let a = create_signal(1);
        let b = create_signal(10);
        let log = Rc::new(RefCell::new(Vec::new()));
        let (l, a2) = (log.clone(), a.clone());
        let _e = create_effect(move || l.borrow_mut().push(a2.get()));
        a.set(2);
        b.set(11); // 読んでいない→再実行しない
        a.set(3);
        assert_eq!(*log.borrow(), vec![1, 2, 3]);
    }

    #[test]
    fn dependencies_follow_branches() {
        let flag = create_signal(true);
        let x = create_signal(1);
        let y = create_signal(100);
        let log = Rc::new(RefCell::new(Vec::new()));
        let (l, f, x2, y2) = (log.clone(), flag.clone(), x.clone(), y.clone());
        let _e = create_effect(move || l.borrow_mut().push(if f.get() { x2.get() } else { y2.get() }));
        y.set(101); // 今は読んでいない
        flag.set(false);
        x.set(2); // もう読んでいない
        y.set(102);
        assert_eq!(*log.borrow(), vec![1, 101, 102]);
    }

    #[test]
    fn memo_notifies_only_when_result_changes() {
        let n = create_signal(1);
        let n2 = n.clone();
        let parity = create_memo(move || n2.get() % 2);
        let log = Rc::new(RefCell::new(Vec::new()));
        let (l, p) = (log.clone(), parity.clone());
        let _e = create_effect(move || l.borrow_mut().push(p.get()));
        n.set(3); // 偶奇は同じ→購読者は起きない
        n.set(4);
        assert_eq!(*log.borrow(), vec![1, 0]);
    }

    #[test]
    fn batch_runs_each_effect_once_and_nests() {
        let a = create_signal(1);
        let b = create_signal(1);
        let log = Rc::new(RefCell::new(Vec::new()));
        let (l, a2, b2) = (log.clone(), a.clone(), b.clone());
        let _e = create_effect(move || l.borrow_mut().push(a2.get() + b2.get()));
        batch(|| {
            a.set(10);
            batch(|| b.set(20));
            assert_eq!(log.borrow().len(), 1, "batch中は再実行しない");
        });
        assert_eq!(*log.borrow(), vec![2, 30]);
    }

    #[test]
    fn stopped_effect_does_not_run_and_self_update_does_not_loop() {
        let s = create_signal(0);
        let count = Rc::new(Cell::new(0));
        let (s2, c) = (s.clone(), count.clone());
        let e = create_effect(move || {
            c.set(c.get() + 1);
            let v = s2.get();
            if v < 5 {
                s2.set(v + 1); // 自分の更新による再入は無視される
            }
        });
        assert_eq!(count.get(), 1);
        e.stop();
        s.set(9);
        assert_eq!(count.get(), 1);
    }
}
