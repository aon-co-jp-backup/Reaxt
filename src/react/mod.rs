//! RS-React(旧RReact、廃止・統合済み)から取り込んだ仮想DOM・diff・hooks・Appループ。
pub mod app;
pub mod diff;
pub mod hooks;
pub mod vnode;

#[cfg(feature = "dom_bridge")]
pub mod dom_bridge;

pub use app::App;
pub use diff::{AttrsPatch, ChildPatch, Patch};
pub use hooks::{
    provide_context, run_component, take_dirty, use_context, use_effect, use_memo, use_reducer, use_ref, use_state, Cleanup,
    ComponentId, Dispatch, Ref, StateSetter,
};
pub use vnode::{VElement, VNode};

#[cfg(feature = "dom_bridge")]
pub use dom_bridge::{apply_patch, render_to_html, render_to_vnode, ElementRef};
