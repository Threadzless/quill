//! Example that shows how to add custom ECS components to a Quill View.

use std::f32::consts::PI;

use bevy::{
    prelude::*,
    render::{
        camera::Viewport,
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
    ui,
};
use bevy_mod_picking::{
    events::PointerCancel,
    picking_core::{CorePlugin, InteractionPlugin},
    prelude::*,
};
use bevy_quill::prelude::*;
use static_init::dynamic;

fn main() {
    App::new()
        .init_resource::<ViewportInset>()
        .init_resource::<PanelWidth>()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins((CorePlugin, InputPlugin, InteractionPlugin, BevyUiBackend))
        .add_plugins(EventListenerPlugin::<Clicked>::default())
        .add_plugins(QuillPlugin)
        .add_systems(Startup, (setup, setup_view_root))
        .add_event::<Clicked>()
        .add_systems(
            Update,
            (
                bevy::window::close_on_esc,
                rotate,
                update_viewport_inset,
                update_camera_viewport,
            ),
        )
        .run();
}

#[dynamic]
static STYLE_MAIN: StyleHandle = StyleHandle::build(|ss| {
    ss.position(ui::PositionType::Absolute)
        .left(10.)
        .top(10.)
        .bottom(10)
        .right(10.)
        .border(1)
        .border_color("#888")
        .display(ui::Display::Flex)
});

#[dynamic]
static STYLE_ASIDE: StyleHandle = StyleHandle::build(|ss| {
    ss.background_color("#222")
        .display(ui::Display::Flex)
        .padding(8)
        .gap(8)
        .flex_direction(ui::FlexDirection::Column)
        .width(200)
});

#[dynamic]
static STYLE_VSPLITTER: StyleHandle = StyleHandle::build(|ss| {
    ss.background_color("#181818")
        .align_items(ui::AlignItems::Center)
        .justify_content(ui::JustifyContent::Center)
        .display(ui::Display::Flex)
        .width(9)
        .selector(".drag", |ss| ss.background_color("#080808"))
});

#[dynamic]
static STYLE_VSPLITTER_INNER: StyleHandle = StyleHandle::build(|ss| {
    ss.background_color("#282828")
        .display(ui::Display::Flex)
        .width(5)
        .height(ui::Val::Percent(30.))
        .pointer_events(PointerEvents::None)
        .selector(":hover > &", |ss| ss.background_color("#383838"))
        .selector(".drag > &", |ss| ss.background_color("#484848"))
});

#[dynamic]
static STYLE_BUTTON: StyleHandle = StyleHandle::build(|ss| {
    ss.background_color("#282828")
        .border_color("#383838")
        .border(1)
        .display(ui::Display::Flex)
        .justify_content(JustifyContent::Center)
        .align_items(AlignItems::Center)
        .min_height(32)
        .padding_left(8)
        .padding_right(8)
        .selector(".pressed", |ss| ss.background_color("#404040"))
        .selector(":hover", |ss| {
            ss.border_color("#444").background_color("#2F2F2F")
        })
        .selector(":hover.pressed", |ss| ss.background_color("#484848"))
});

#[dynamic]
static STYLE_VIEWPORT: StyleHandle = StyleHandle::build(|ss| ss.flex_grow(1.));

const DEFAULT_FOV: f32 = 0.69; // 40 degrees
const X_EXTENT: f32 = 14.5;
const CLS_DRAG: &str = "drag";
const CLS_PRESSED: &str = "pressed";

/// A marker component for our shapes so we can query them separately from the ground plane
#[derive(Component)]
struct Shape;

/// Marker which identifies the primary camera.
#[derive(Component)]
pub struct PrimaryCamera;

/// Used to create margins around the viewport so that side panels don't overwrite the 3d scene.
#[derive(Default, Resource, PartialEq)]
pub struct ViewportInset {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

/// A marker component for that identifies which element contains the 3d view. The
/// `update_viewport_inset` system measures the on-screen position of the UiNode that this
/// component is attached to, and updates the screen position of the 3D view to match it.
#[derive(Component, Clone)]
pub struct ViewportInsetElement;

#[derive(Resource)]
pub struct PanelWidth(pub i32);

impl Default for PanelWidth {
    fn default() -> Self {
        Self(160)
    }
}

fn setup_view_root(mut commands: Commands) {
    let camera2d = commands
        .spawn((Camera2dBundle {
            camera: Camera {
                // HUD goes on top of 3D
                order: 1,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            ..default()
        },))
        .id();

    commands.spawn((TargetCamera(camera2d), ViewHandle::new(ui_main, ())));
}

fn ui_main(cx: Cx) -> impl View {
    let width = cx.use_resource::<PanelWidth>();
    Element::new().styled(STYLE_MAIN.clone()).children((
        Element::new()
            .styled((
                STYLE_ASIDE.clone(),
                StyleHandle::build(|b| b.width(width.0)),
            ))
            .children((
                button.bind(ButtonProps {
                    id: "save",
                    children: "Save",
                }),
                button.bind(ButtonProps {
                    id: "load",
                    children: "Load",
                }),
                button.bind(ButtonProps {
                    id: "quit",
                    children: "Quit",
                }),
            ))
            .insert((On::<Clicked>::run(|ev: Listener<Clicked>| {
                println!("Clicked {}", ev.id);
            }),)),
        v_splitter,
        Element::new()
            .styled(STYLE_VIEWPORT.clone())
            .insert(ViewportInsetElement {}),
    ))
}

fn v_splitter(_cx: Cx) -> impl View {
    Element::new()
        .children(Element::new().styled(STYLE_VSPLITTER_INNER.clone()))
        .insert((
            On::<Pointer<DragStart>>::listener_component_mut::<ElementClasses>(|_, classes| {
                classes.add_class(CLS_DRAG)
            }),
            On::<Pointer<DragEnd>>::listener_component_mut::<ElementClasses>(|_, classes| {
                classes.remove_class(CLS_DRAG)
            }),
            On::<Pointer<Drag>>::run(|ev: Listener<Pointer<Drag>>, mut res: ResMut<PanelWidth>| {
                res.0 += ev.delta.x as i32;
            }),
            On::<Pointer<PointerCancel>>::listener_component_mut::<ElementClasses>(|_, classes| {
                println!("Cancel");
                classes.remove_class(CLS_DRAG)
            }),
        ))
        .styled(STYLE_VSPLITTER.clone())
}

#[derive(Clone, PartialEq)]
struct ButtonProps<V: View> {
    id: &'static str,
    children: V,
}

#[derive(Clone, Event, EntityEvent)]
#[can_bubble]
struct Clicked {
    #[target]
    target: Entity,
    id: &'static str,
}

fn button<V: View + Clone>(cx: Cx<ButtonProps<V>>) -> impl View {
    // Needs to be a local variable so that it can be captured in the event handler.
    let id = cx.props.id;
    Element::new()
        .children(cx.props.children.clone())
        .insert((
            On::<Pointer<Click>>::run(
                move |events: Listener<Pointer<Click>>, mut ev: EventWriter<Clicked>| {
                    ev.send(Clicked {
                        target: events.target,
                        id,
                    });
                },
            ),
            On::<Pointer<DragStart>>::listener_component_mut::<ElementClasses>(|_, classes| {
                classes.add_class(CLS_PRESSED)
            }),
            On::<Pointer<DragEnd>>::listener_component_mut::<ElementClasses>(|_, classes| {
                classes.remove_class(CLS_PRESSED)
            }),
            On::<Pointer<PointerCancel>>::listener_component_mut::<ElementClasses>(|_, classes| {
                println!("Cancel");
                classes.remove_class(CLS_PRESSED)
            }),
        ))
        .styled(STYLE_BUTTON.clone())
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        ..default()
    });

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 6., 12.0)
                .looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
            ..default()
        },
        PrimaryCamera,
    ));

    // ground plane
    commands.spawn(
        PbrBundle {
            mesh: meshes.add(Plane3d::default().mesh().size(50.0, 50.0)),
            material: materials.add(Color::SILVER),
            ..default()
        },
    );

    let shapes = [
        meshes.add(Cuboid::default().mesh().scaled_by(Vec3::new(1.0, 1.0, 1.0))),
        meshes.add(Cuboid::default().mesh().scaled_by(Vec3::new(1.0, 2.0, 1.0))),
        meshes.add(Capsule3d::default().mesh()),
        meshes.add(Torus::default().mesh()),
        meshes.add(Cylinder::default().mesh()),
        meshes.add(Sphere::default().mesh().ico(5).unwrap()),
        meshes.add(Sphere::default().mesh().uv(32, 18)),
    ];

    let num_shapes = shapes.len();

    for (i, shape) in shapes.into_iter().enumerate() {
        commands.spawn((
            PbrBundle {
                mesh: shape,
                material: debug_material.clone(),
                transform: Transform::from_xyz(
                    -X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * X_EXTENT,
                    2.0,
                    0.0,
                )
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
                ..default()
            },
            Shape,
        ));
    }

    commands.spawn(
        PointLightBundle {
            point_light: PointLight {
                intensity: 9_000_000.0,
                range: 100.,
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_xyz(8.0, 16.0, 8.0),
            ..default()
        },
    );
}

pub fn update_viewport_inset(
    windows: Query<&Window>,
    query: Query<(&Node, &GlobalTransform), With<ViewportInsetElement>>,
    mut viewport_inset: ResMut<ViewportInset>,
) {
    let mut inset = ViewportInset::default();
    match query.get_single() {
        Ok((node, transform)) => {
            let position = transform.translation();
            let ui_position = position.truncate();
            let extents = node.size() / 2.0;
            let min = ui_position - extents;
            let max = ui_position + extents;

            let window = windows.single();
            let ww = window.resolution.physical_width() as f32;
            let wh = window.resolution.physical_height() as f32;
            let sf = window.resolution.scale_factor() as f32;

            inset.left = min.x;
            inset.top = min.y;
            inset.right = ww / sf - max.x;
            inset.bottom = wh / sf - max.y;
        }
        Err(_) => {
            if query.iter().count() > 1 {
                error!("Multiple ViewportInsetControllers!");
            }
        }
    }

    if inset != *viewport_inset {
        *viewport_inset.as_mut() = inset;
    }
}

/// Update the camera viewport and fov properties based on the window size and the viewport
/// margins.
pub fn update_camera_viewport(
    viewport_inset: Res<ViewportInset>,
    windows: Query<&Window>,
    mut camera_query: Query<(&mut Camera, &mut Projection), With<PrimaryCamera>>,
) {
    let window = windows.single();
    let ww = window.resolution.physical_width() as f32;
    let wh = window.resolution.physical_height() as f32;
    let sf = window.resolution.scale_factor() as f32;
    let left = viewport_inset.left * sf;
    let right = viewport_inset.right * sf;
    let top = viewport_inset.top * sf;
    let bottom = viewport_inset.bottom * sf;
    let vw = (ww - left - right).max(1.);
    let vh = (wh - top - bottom).max(1.);

    let (mut camera, mut projection) = camera_query.single_mut();
    camera.viewport = Some(Viewport {
        physical_position: UVec2::new(left as u32, top as u32),
        physical_size: UVec2::new(vw as u32, vh as u32),
        ..default()
    });

    if let Projection::Perspective(ref mut perspective) = *projection {
        let aspect = vw / vh;
        perspective.aspect_ratio = aspect;
        perspective.fov = f32::min(DEFAULT_FOV, DEFAULT_FOV * 2. / aspect);
        perspective.near = 0.5;
        perspective.far = 100.;
    }
}

fn rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_seconds() / 2.);
    }
}

/// Creates a colorful test pattern
fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102, 255,
        198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}
