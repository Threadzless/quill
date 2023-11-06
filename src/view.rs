use std::marker::PhantomData;

use bevy::{
    prelude::*,
    text::{Text, TextStyle},
};

use crate::ViewHandle;

use super::node_span::NodeSpan;

pub struct ElementContext<'w> {
    pub(crate) world: &'w mut World,
    pub(crate) entity: Entity,
}

pub trait AnyResource: Send + Sync {
    fn is_changed(&self, world: &World) -> bool;
}

#[derive(PartialEq, Eq)]
pub struct AnyRes<T> {
    pub pdata: PhantomData<T>,
}

impl<T> AnyRes<T> {
    fn new() -> Self {
        Self { pdata: PhantomData }
    }
}

impl<T> AnyResource for AnyRes<T>
where
    T: Resource,
{
    fn is_changed(&self, world: &World) -> bool {
        world.is_resource_changed::<T>()
    }
}

/// Tracks resources used by each ViewState
#[derive(Component, Default)]
pub struct TrackedResources {
    pub data: Vec<Box<dyn AnyResource>>,
}

/// Cx is a context parameter that is passed to presenters. It contains the presenter's
/// properties (passed from the parent presenter), plus other context information needed
/// in building the view state graph.
// TODO: Move this to it's own file once it's stable.
pub struct Cx<'w, 'p, Props = ()> {
    pub props: &'p Props,
    pub sys: &'p mut ElementContext<'w>,
}

impl<'w, 'p, Props> Cx<'w, 'p, Props> {
    pub fn use_resource<T: Resource>(&mut self) -> &T {
        let mut tracked = self
            .sys
            .world
            .get_mut::<TrackedResources>(self.sys.entity)
            .expect("TrackedResources not found for this entity");
        tracked.data.push(Box::new(AnyRes::<T>::new()));
        self.sys.world.resource::<T>()
    }

    pub fn use_resource_mut<T: Resource>(&mut self) -> Mut<T> {
        let mut tracked = self
            .sys
            .world
            .get_mut::<TrackedResources>(self.sys.entity)
            .expect("TrackedResources not found for this entity");
        tracked.data.push(Box::new(AnyRes::<T>::new()));
        self.sys.world.resource_mut::<T>()
    }
}

pub trait View: Send + Sync {
    type State: Send + Sync + Default;

    /// Construct and patch the tree of UiNodes produced by this view.
    /// This may also spawn child entities representing nested components.
    fn build(&self, ecx: &mut ElementContext, state: &mut Self::State, prev: &NodeSpan)
        -> NodeSpan;

    /// Recursively despawn any child entities that were created as a result of calling `.build()`.
    /// This calls `.raze()` for any nested views within the current view state.
    fn raze(&self, _ecx: &mut ElementContext, _state: &mut Self::State, prev: &NodeSpan);
}

/// View which renders nothing
impl View for () {
    type State = ();

    fn build(
        &self,
        _ecx: &mut ElementContext,
        _state: &mut Self::State,
        _prev: &NodeSpan,
    ) -> NodeSpan {
        NodeSpan::Empty
    }

    fn raze(&self, _ecx: &mut ElementContext, _state: &mut Self::State, _nodes: &NodeSpan) {}
}

/// View which renders a String
impl View for String {
    type State = ();

    fn build(
        &self,
        ecx: &mut ElementContext,
        _state: &mut Self::State,
        prev: &NodeSpan,
    ) -> NodeSpan {
        if let NodeSpan::Node(text_entity) = prev {
            if let Some(mut old_text) = ecx.world.entity_mut(*text_entity).get_mut::<Text>() {
                // TODO: compare text for equality.
                old_text.sections.clear();
                old_text.sections.push(TextSection {
                    value: self.to_owned(),
                    style: TextStyle { ..default() },
                });
                return NodeSpan::Node(*text_entity);
            }
        }

        prev.despawn_recursive(ecx.world);
        let new_entity = ecx
            .world
            .spawn((TextBundle {
                text: Text::from_section(self.clone(), TextStyle { ..default() }),
                // TextStyle {
                //     font_size: 40.0,
                //     color: Color::rgb(0.9, 0.9, 0.9),
                //     ..Default::default()
                // },
                // background_color: Color::rgb(0.65, 0.75, 0.65).into(),
                // border_color: Color::BLUE.into(),
                // focus_policy: FocusPolicy::Pass,
                ..default()
            },))
            .id();

        return NodeSpan::Node(new_entity);
    }

    fn raze(&self, ecx: &mut ElementContext, _state: &mut Self::State, prev: &NodeSpan) {
        prev.despawn_recursive(ecx.world);
        // ecx.world.entity_mut(ecx.entity).despawn_recursive();
    }
}

/// View which renders a string slice.
impl View for &'static str {
    type State = ();

    fn build(
        &self,
        ecx: &mut ElementContext,
        _state: &mut Self::State,
        prev: &NodeSpan,
    ) -> NodeSpan {
        if let NodeSpan::Node(text_entity) = prev {
            if let Some(mut old_text) = ecx.world.entity_mut(*text_entity).get_mut::<Text>() {
                // TODO: compare text for equality.
                old_text.sections.clear();
                old_text.sections.push(TextSection {
                    value: self.to_string(),
                    style: TextStyle { ..default() },
                });
                return NodeSpan::Node(*text_entity);
            }
        }

        prev.despawn_recursive(ecx.world);
        let new_entity = ecx
            .world
            .spawn((TextBundle {
                text: Text::from_section(self.to_string(), TextStyle { ..default() }),
                // TextStyle {
                //     font_size: 40.0,
                //     color: Color::rgb(0.9, 0.9, 0.9),
                //     ..Default::default()
                // },
                // background_color: Color::rgb(0.65, 0.75, 0.65).into(),
                // border_color: Color::BLUE.into(),
                // focus_policy: FocusPolicy::Pass,
                ..default()
            },))
            .id();

        return NodeSpan::Node(new_entity);
    }

    fn raze(&self, ecx: &mut ElementContext, _state: &mut Self::State, prev: &NodeSpan) {
        prev.despawn_recursive(ecx.world);
        // ecx.world.entity_mut(ecx.entity).despawn_recursive();
    }
}

/// View which renders a bare presenter with no arguments
impl<A: View + 'static> View for fn(cx: Cx) -> A {
    type State = Option<Entity>;

    fn build(
        &self,
        parent_ecx: &mut ElementContext,
        state: &mut Self::State,
        prev: &NodeSpan,
    ) -> NodeSpan {
        let mut child_state: A::State = Default::default();
        let entity: Entity = match state {
            Some(entity) => *entity,
            None => {
                let entity = parent_ecx
                    .world
                    .spawn(TrackedResources::default())
                    .set_parent(parent_ecx.entity)
                    .id();
                *state = Some(entity);
                entity
            }
        };
        let mut child_context = ElementContext {
            world: parent_ecx.world,
            entity,
        };
        let cx = Cx {
            sys: &mut child_context,
            props: &(),
        };
        self(cx).build(parent_ecx, &mut child_state, prev)
    }

    fn raze(&self, _ecx: &mut ElementContext, _state: &mut Self::State, _prev: &NodeSpan) {
        todo!();
    }
}

/// Binds a presenter to properties and implements a view
pub struct Bind<V: View, Props: Send + Sync + Clone> {
    presenter: fn(cx: Cx<Props>) -> V,
    props: Props,
}

impl<V: View, Props: Send + Sync + Clone> Bind<V, Props> {
    pub fn new(presenter: fn(cx: Cx<Props>) -> V, props: Props) -> Self {
        Self { presenter, props }
    }
}

impl<V: View + 'static, Props: Send + Sync + 'static + Clone> View for Bind<V, Props> {
    type State = Option<Entity>;

    fn build(
        &self,
        parent_ecx: &mut ElementContext,
        state: &mut Self::State,
        prev: &NodeSpan,
    ) -> NodeSpan {
        let entity = match state {
            Some(entity) => *entity,
            None => {
                let entity = parent_ecx
                    .world
                    .spawn((
                        TrackedResources::default(),
                        ViewHandle::new(self.presenter, self.props.clone()),
                    ))
                    .set_parent(parent_ecx.entity)
                    .id();
                *state = Some(entity);
                entity
            }
        };

        // get the handle from the current view state
        let mut entt = parent_ecx.world.entity_mut(entity);
        let Some(mut handle) = entt.get_mut::<ViewHandle>() else {
            return NodeSpan::Empty;
        };
        let mut inner = handle
            .inner
            .take()
            .expect("ViewState::handle should be present at this point");

        let mut child_context = ElementContext {
            world: parent_ecx.world,
            entity,
        };

        // build the view
        inner.build(&mut child_context, entity);
        let nodes = inner.nodes(prev);

        // put back the handle
        let mut entt = parent_ecx.world.entity_mut(entity);
        let Some(mut view_state) = entt.get_mut::<ViewHandle>() else {
            return NodeSpan::Empty;
        };
        view_state.inner = Some(inner);

        nodes
    }

    fn raze(&self, ecx: &mut ElementContext, state: &mut Self::State, _prev: &NodeSpan) {
        if let Some(entity) = state.take() {
            let mut entt = ecx.world.entity_mut(entity);
            let Some(mut handle) = entt.get_mut::<ViewHandle>() else {
                return;
            };
            let mut inner = handle
                .inner
                .take()
                .expect("ViewState::handle should be present at this point");
            inner.raze(ecx, entity)
        }
    }
}
