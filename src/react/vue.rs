//! Vue 風の呼び名(`ref` / `computed` / `watch` / `watch_effect`)。実体は `signals` と同じ仕組みで、
//! 別の機構は持たない(Vueの `ref`・`computed`・`watchEffect` に相当)。`reactive`(オブジェクトの深い監視)は
//! Rustの型に合わないので作らない: 構造体は `Signal<構造体>` に入れるか、フィールドごとに `ref_` を持つ。
use super::signals::{create_effect, create_memo, create_signal, Effect, Memo, Signal};

/// Vueの `ref(v)`。`ref` はRustの予約語なので `ref_`。読む=`.get()`、書く=`.set()`。
pub fn ref_<T: Clone + 'static>(v: T) -> Signal<T> {
    create_signal(v)
}

/// Vueの `computed(() => ...)`。
pub fn computed<T: Clone + PartialEq + 'static>(f: impl FnMut() -> T + 'static) -> Memo<T> {
    create_memo(f)
}

/// Vueの `watchEffect(fn)`。
pub fn watch_effect(f: impl FnMut() + 'static) -> Effect {
    create_effect(f)
}

/// Vueの `watch(source, cb)`。`source` の値が変わるたび `cb(新, 旧)` を呼ぶ(最初の1回は呼ばない)。
pub fn watch<T: Clone + PartialEq + 'static>(source: impl Fn() -> T + 'static, mut cb: impl FnMut(&T, &T) + 'static) -> Effect {
    let prev: std::rc::Rc<std::cell::RefCell<Option<T>>> = Default::default();
    create_effect(move || {
        let now = source();
        let old = prev.borrow_mut().replace(now.clone());
        if let Some(old) = old {
            if old != now {
                cb(&now, &old);
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn computed_and_watch_follow_ref() {
        let n = ref_(1);
        let n2 = n.clone();
        let double = computed(move || n2.get() * 2);
        let log = Rc::new(RefCell::new(Vec::new()));
        let (l, d) = (log.clone(), double.clone());
        let _w = watch(move || d.get(), move |new, old| l.borrow_mut().push((*old, *new)));
        n.set(2);
        n.set(5);
        assert_eq!(*log.borrow(), vec![(2, 4), (4, 10)]);
    }
}
