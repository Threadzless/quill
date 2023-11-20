use bevy::{
    prelude::*,
    text::{Text, TextStyle},
};

use crate::{Cx, ViewHandle};

use crate::node_span::NodeSpan;

use super::{
    cx::ElementContext,
    view_insert::ViewInsert,
    view_styled::{StyleTuple, ViewStyled},
    view_with::ViewWith,
};

pub trait View: Send + Sync
where
    Self: Sized,
{
    type State: Send + Sync + Default;

    /// Construct and patch the tree of UiNodes produced by this view.
    /// This may also spawn child entities representing nested components.
    fn build(&self, ecx: &mut ElementContext, state: &mut Self::State, prev: &NodeSpan)
        -> NodeSpan;

    /// Recursively despawn any child entities that were created as a result of calling `.build()`.
    /// This calls `.raze()` for any nested views within the current view state.
    fn raze(&self, _ecx: &mut ElementContext, state: &mut Self::State, prev: &NodeSpan);

    /// Apply styles to this view.
    fn styled<S: StyleTuple>(self, styles: S) -> ViewStyled<Self> {
        ViewStyled::new(self, styles)
    }

    /// Inserts a default instance of the specified component to the nodes generated by this view,
    /// if it's not already inserted.
    fn insert<C: Component + Clone>(self, component: C) -> ViewInsert<Self, C> {
        ViewInsert {
            inner: self,
            component,
        }
    }

    /// Sets up a callback which is called for each output UiNode. Typically used to manipulate
    /// components on the entity. This is called each time the view is rebuilt.
    fn with<F: Fn(Entity, &mut World) -> () + Send + Sync>(self, callback: F) -> ViewWith<Self, F> {
        ViewWith {
            inner: self,
            callback,
            once: false,
        }
    }

    /// Sets up a callback which is called for each output UiNode, but only when the node is first
    /// created.
    fn once<F: Fn(Entity, &mut World) -> () + Send + Sync>(self, callback: F) -> ViewWith<Self, F> {
        ViewWith {
            inner: self,
            callback,
            once: true,
        }
    }
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
        println!("Raze: String {}", self);
        prev.despawn_recursive(ecx.world);
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
        println!("Raze: &str {}", self);
        prev.despawn_recursive(ecx.world);
    }
}

/// View which renders a bare presenter with no arguments
impl<V: View + 'static, F: Fn(Cx<()>) -> V + Send + Sync + Copy + 'static> View for F {
    type State = Option<Entity>;

    fn build(
        &self,
        parent_ecx: &mut ElementContext,
        state: &mut Self::State,
        prev: &NodeSpan,
    ) -> NodeSpan {
        let entity: Entity = match state {
            Some(entity) => *entity,
            None => {
                let entity = parent_ecx
                    .world
                    .spawn(ViewHandle::new(*self, ()))
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

/// Binds a presenter to properties and implements a view
pub struct Bind<
    V: View,
    Props: Send + Sync + Clone,
    F: FnMut(Cx<Props>) -> V + Send + Sync + Copy + 'static,
> {
    presenter: F,
    props: Props,
}

impl<
        V: View,
        Props: Send + Sync + Clone,
        F: FnMut(Cx<Props>) -> V + Send + Sync + Copy + 'static,
    > Bind<V, Props, F>
{
    pub fn new(presenter: F, props: Props) -> Self {
        Self { presenter, props }
    }
}

impl<
        V: View + 'static,
        Props: Send + Sync + 'static + Clone,
        F: FnMut(Cx<Props>) -> V + Send + Sync + Copy + 'static,
    > View for Bind<V, Props, F>
{
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
                    .spawn(ViewHandle::new(self.presenter, self.props.clone()))
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

pub trait PresenterFn<
    V: View,
    Props: Send + Sync + Clone,
    F: FnMut(Cx<Props>) -> V + Send + Sync + Copy + 'static,
>
{
    fn bind(self, props: Props) -> Bind<V, Props, F>;
}

impl<
        V: View,
        Props: Send + Sync + Clone,
        F: FnMut(Cx<Props>) -> V + Send + Sync + Copy + 'static,
    > PresenterFn<V, Props, F> for F
{
    fn bind(self, props: Props) -> Bind<V, Props, Self> {
        Bind::new(self, props)
    }
}
