use bevy::{
    a11y::Focus,
    prelude::*,
    render::texture::ImageLoaderSettings,
};
use bevy_mod_picking::focus::{HoverMap, PreviousHoverMap};

use crate::{
    style::{ComputedStyle, UpdateComputedStyle}, ElementClasses, ElementStyles, QuillPlugin, SelectorMatcher
};

use super::{computed::ComputedImage, style_handle::TextStyles};

#[derive(Resource, Default)]
pub(crate) struct PreviousFocus(Option<Entity>);

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub(crate) fn update_styles(
    mut commands: Commands,
    query_root: Query<Entity, (With<Node>, Without<Parent>)>,
    query_styles: Query<
        (
            Ref<Style>,
            Option<Ref<ElementStyles>>,
            Option<&TextStyles>,
            Option<Ref<Text>>,
        ),
        With<Node>,
    >,
    query_element_classes: Query<Ref<'static, ElementClasses>>,
    query_parents: Query<&'static Parent, (With<Node>, With<Visibility>)>,
    query_children: Query<&'static Children, (With<Node>, With<Visibility>)>,
    hover_map: Res<HoverMap>,
    hover_map_prev: Res<PreviousHoverMap>,
    assets: Res<AssetServer>,
    focus: Res<Focus>,
    plugin: Res<QuillPlugin>,
    mut focus_prev: ResMut<PreviousFocus>,
) {
    let matcher = SelectorMatcher::new(
        &query_element_classes,
        &query_parents,
        &query_children,
        &hover_map.0,
        focus.0,
    );
    let matcher_prev = SelectorMatcher::new(
        &query_element_classes,
        &query_parents,
        &query_children,
        &hover_map_prev.0,
        focus_prev.0,
    );

    for root_node in &query_root {
        update_element_styles(
            &mut commands,
            &query_styles,
            &query_element_classes,
            &query_parents,
            &query_children,
            &matcher,
            &matcher_prev,
            &assets,
            root_node,
            &TextStyles::default(),
            &plugin,
            false,
        )
    }

    focus_prev.0 = focus.0;
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn update_element_styles(
    commands: &mut Commands,
    query_styles: &Query<
        (
            Ref<Style>,
            Option<Ref<ElementStyles>>,
            Option<&TextStyles>,
            Option<Ref<Text>>,
        ),
        With<Node>,
    >,
    classes_query: &Query<Ref<'static, ElementClasses>>,
    parent_query: &Query<'_, '_, &Parent, (With<Node>, With<Visibility>)>,
    children_query: &Query<'_, '_, &Children, (With<Node>, With<Visibility>)>,
    matcher: &SelectorMatcher<'_, '_, '_>,
    matcher_prev: &SelectorMatcher<'_, '_, '_>,
    assets: &Res<AssetServer>,
    entity: Entity,
    inherited_styles: &TextStyles,
    plugin: &QuillPlugin,
    mut inherited_styles_changed: bool,
) {
    let mut text_styles = inherited_styles.clone();

    if let Ok((style, elt_styles, prev_text_styles, txt)) = query_styles.get(entity) {
        // Check if the element styles or ancestor classes have changed.
        let mut changed = match elt_styles {
            Some(ref element_style) => is_changed(
                element_style,
                entity,
                classes_query,
                matcher,
                matcher_prev,
                parent_query,
            ),
            None => false,
        };

        if let Some(ref text_node) = txt {
            if text_node.is_changed() {
                changed = true;
            }
        }

        if changed || inherited_styles_changed {
            // Compute computed style. Initialize to the current state.
            let mut computed = ComputedStyle::new();
            computed.style = style.clone();

            // Inherited properties
            computed.font_handle = inherited_styles.font.clone();
            computed.font_size = inherited_styles.font_size;
            computed.color = inherited_styles.color;

            // Apply element styles to computed
            if let Some(ref element_styles) = elt_styles {
                for ss in element_styles.styles.iter() {
                    ss.apply_to(&mut computed, matcher, &entity);
                }
                // Load font asset if non-null.
                if let Some(ref font_path) = computed.font {
                    computed.font_handle = Some(assets.load(font_path));
                }
            }

            // Update inherited text styles
            text_styles.font = computed.font_handle.clone();
            text_styles.font_size = computed.font_size;
            text_styles.color = computed.color;

            if text_styles == *inherited_styles && txt.is_none() {
                // No change from parent, so we can remove the cached styles and rely on inherited
                // styles only. Note that for text nodes, we always want to store the inherited
                // styles, even if they are the same as the parent.
                inherited_styles_changed = prev_text_styles.is_some();
                if inherited_styles_changed {
                    changed = true;
                    commands.entity(entity).remove::<TextStyles>();
                }
            } else {
                // Text styles are different from parent, so we need to store a cached copy.
                inherited_styles_changed = prev_text_styles != Some(&text_styles);
                if inherited_styles_changed {
                    changed = true;
                    commands.entity(entity).insert(text_styles.clone());
                }
            }

            if changed {
                computed.image_handle = match computed.image.as_ref() {
                    None => None,
                    Some(ComputedImage::Handle(h)) => Some(h.clone()),
                    Some(ComputedImage::Path(p)) => {
                        let sampler = plugin.default_sampler.clone();
                        Some(
                            assets.load_with_settings(p, move |s: &mut ImageLoaderSettings| {
                                s.sampler = sampler.clone()
                            })
                        )
                    }
                };
                
                commands.add(UpdateComputedStyle { entity, computed });
            }
        } else if let Some(prev) = prev_text_styles {
            // Styles didn't change, but we need to pass inherited text styles to children.
            text_styles = prev.clone();
        }
    }

    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            update_element_styles(
                commands,
                query_styles,
                classes_query,
                parent_query,
                children_query,
                matcher,
                matcher_prev,
                assets,
                *child,
                &text_styles,
                plugin,
                inherited_styles_changed,
            );
        }
    }
}

/// Detects whether the given entity's styles have changed, or whether any of its ancestors
/// have changed in a way that would affect the computation of styles (either because
/// of class list changes or hovering).
fn is_changed(
    element_styles: &Ref<'_, ElementStyles>,
    entity: Entity,
    classes_query: &Query<Ref<'static, ElementClasses>>,
    matcher: &SelectorMatcher<'_, '_, '_>,
    matcher_prev: &SelectorMatcher<'_, '_, '_>,
    parent_query: &Query<'_, '_, &Parent, (With<Node>, With<Visibility>)>,
) -> bool {
    // Style changes only affect current element, not children.
    let mut changed = element_styles.is_changed();

    // Search ancestors to see if any have changed.
    // We want to know if either the class list or the hover state has changed.
    if !changed && element_styles.selector_depth > 0 {
        let mut e = entity;
        for _ in 0..element_styles.selector_depth {
            if let Ok(a_classes) = classes_query.get(e) {
                if element_styles.uses_hover
                    && matcher.is_hovering(&e) != matcher_prev.is_hovering(&e)
                {
                    changed = true;
                    break;
                }

                if matcher.is_focused(&e) != matcher_prev.is_focused(&e) {
                    changed = true;
                    break;
                }

                if matcher.is_focus_visible(&e) != matcher_prev.is_focus_visible(&e) {
                    changed = true;
                    break;
                }

                if element_styles.uses_focus_within
                    && matcher.is_focus_within(&e) != matcher_prev.is_focus_within(&e)
                {
                    changed = true;
                    break;
                }

                if a_classes.is_changed() {
                    changed = true;
                    break;
                }
            }

            match parent_query.get(e) {
                Ok(parent) => e = **parent,
                _ => break,
            }
        }
    }
    changed
}
