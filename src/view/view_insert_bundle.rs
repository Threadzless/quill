use std::cell::Cell;

use bevy::prelude::*;

use crate::{BuildContext, View};

use crate::node_span::NodeSpan;

/// An implementtion of [`View`] that inserts an ECS Component on the generated display entities.
///
/// The Component will only be inserted once on an entity. This happens when the entity is
/// first created, and also will happen if the output entity is replaced by a different entity.
pub struct ViewInsertBundle<V: View, B: Bundle> {
    pub(crate) inner: V,
    pub(crate) component: Cell<Option<B>>,
}

impl<V: View, B: Bundle> ViewInsertBundle<V, B> {
    fn insert_component(&self, nodes: &NodeSpan, vc: &mut BuildContext) {
        match nodes {
            NodeSpan::Empty => (),
            NodeSpan::Node(entity) => {
                let em = &mut vc.entity_mut(*entity);
                em.insert(self.component.take().unwrap());
            }
            NodeSpan::Fragment(ref _nodes) => {
                panic!("Can only insert into a singular node")
            }
        }
    }
}

impl<V: View, B: Bundle> View for ViewInsertBundle<V, B> {
    type State = (V::State, NodeSpan);

    fn nodes(&self, vc: &BuildContext, state: &Self::State) -> NodeSpan {
        self.inner.nodes(vc, &state.0)
    }

    fn build(&self, vc: &mut BuildContext) -> Self::State {
        let state = self.inner.build(vc);
        let mut nodes = self.inner.nodes(vc, &state);
        self.insert_component(&mut nodes, vc);
        (state, nodes)
    }

    fn update(&self, vc: &mut BuildContext, state: &mut Self::State) {
        self.inner.update(vc, &mut state.0);
        let nodes = self.inner.nodes(vc, &state.0);
        // Only insert the component when the output entity has changed.
        if state.1 != nodes {
            state.1 = nodes;
            self.insert_component(&mut state.1, vc);
        }
    }

    fn assemble(&self, vc: &mut BuildContext, state: &mut Self::State) -> NodeSpan {
        self.inner.assemble(vc, &mut state.0)
    }

    fn raze(&self, world: &mut World, state: &mut Self::State) {
        self.inner.raze(world, &mut state.0);
    }
}
