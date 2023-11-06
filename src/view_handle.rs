use bevy::prelude::*;

use super::{
    view::{Cx, ElementContext},
    NodeSpan, View,
};

/// A ViewHandle holds a type-erased reference to a presenter function and its props and state.
#[derive(Component)]
pub struct ViewHandle {
    pub inner: Option<Box<dyn AnyViewState>>,
}

impl ViewHandle {
    /// Construct a new ViewRoot from a presenter and props.
    pub fn new<V: View + 'static, Props: Send + Sync + 'static + Clone>(
        presenter: fn(cx: Cx<Props>) -> V,
        props: Props,
    ) -> Self {
        Self {
            inner: Some(Box::new(ViewState::new(presenter, props))),
        }
    }

    /// Return the count of top-level UiNodes
    pub fn count(&self) -> usize {
        self.inner.as_ref().unwrap().count()
    }

    // /// Rebuild the UiNodes.
    // pub fn build(&mut self, world: &mut World, entity: Entity) {
    //     let mut ec = ElementContext { world, entity };
    //     self.inner.as_mut().unwrap().build(&mut ec, entity);
    // }
}

/// `ViewState` contains all of the data needed to re-render a presenter: The presenter function,
/// its properties, its state, and the cached output nodes.
///
/// This type is generic on the props and state for the presenter.
pub struct ViewState<V: View, Props: Send + Sync> {
    /// Reference to presenter function
    presenter: fn(cx: Cx<Props>) -> V,

    /// Props passed to the presenter
    props: Props,

    /// View tree output by presenter
    view: Option<V>,

    /// Externalized state defined by view tree
    state: V::State,

    /// The UiNodes generated by this view state
    nodes: NodeSpan,
}

impl<V: View, Props: Send + Sync> ViewState<V, Props> {
    pub fn new(presenter: fn(cx: Cx<Props>) -> V, props: Props) -> Self {
        Self {
            presenter,
            nodes: NodeSpan::Empty,
            props,
            view: None,
            state: Default::default(),
        }
    }
}

/// `AnyViewState` is a type-erased version of `ViewState`. It allows holding a reference
/// to a renderable presenter without knowing the type of its props and state.
pub trait AnyViewState: Send + Sync {
    // Return the number of top-level UiNodes generated by this view.
    fn count(&self) -> usize;

    // Return the nodes that were generated by this view.
    fn nodes(&self, prev: &NodeSpan) -> NodeSpan;

    // Rebuild the NodeSpans for this view and update the state.
    fn build(&mut self, cx: &mut ElementContext, entity: Entity);

    // Release all state and despawn all child entities.
    fn raze(&mut self, cx: &mut ElementContext, entity: Entity);
}

impl<V: View, Props: Send + Sync + Clone> AnyViewState for ViewState<V, Props> {
    fn count(&self) -> usize {
        self.nodes.count()
    }

    fn build(&mut self, ecx: &mut ElementContext, entity: Entity) {
        let mut child_context = ElementContext {
            world: ecx.world,
            entity,
        };
        let cx = Cx::<Props> {
            sys: &mut child_context,
            props: &self.props,
        };
        self.view = Some((self.presenter)(cx));
        self.nodes =
            self.view
                .as_ref()
                .unwrap()
                .build(&mut child_context, &mut self.state, &self.nodes);
    }

    fn raze(&mut self, ecx: &mut ElementContext, entity: Entity) {
        let mut child_context = ElementContext {
            world: ecx.world,
            entity,
        };
        if let Some(ref view) = self.view {
            view.raze(&mut child_context, &mut self.state, &self.nodes);
            self.view = None;
        }
    }

    fn nodes(&self, _prev: &NodeSpan) -> NodeSpan {
        self.nodes.clone()
    }
}
