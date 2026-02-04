use bevy::asset::AssetPlugin;
use bevy::audio::{AudioPlayer, PlaybackSettings, Volume};
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{RenderTarget, ScalingMode};
use bevy::image::{ImagePlugin, ImageSampler};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::settings::{Backends, PowerPreference, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy::render::view::Msaa;
use bevy::asset::RenderAssetUsages;
use bevy::ui::IsDefaultUiCamera;
use bevy::text::LineHeight;
use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;

const LEVEL_WIDTH: f32 = 1600.0;
const LEVEL_HEIGHT: f32 = 225.0;
const VIEW_WIDTH: f32 = 400.0;
const VIEW_HEIGHT: f32 = 225.0;

const DOOR_OPEN_OFFSET: f32 = 26.0;

const Z_BG: f32 = -20.0;
const Z_WALL: f32 = -15.0;
const Z_VIGNETTE: f32 = -10.0;
const Z_PLATFORM: f32 = 0.0;
const Z_INTERACT: f32 = 1.0;
const Z_ACTOR: f32 = 5.0;
const Z_GLOW: f32 = 2.0;
const Z_LABEL: f32 = 30.0;

const PRESENT_LAYER: usize = 1;
const TORCH_GLOW_Y_OFFSET: f32 = 10.0;
const TORCH_GLOW_BASE_ALPHA: f32 = 0.2;
const TORCH_GLOW_MIN_ALPHA_FACTOR: f32 = 0.5;
const PRINCESS_SCALE: f32 = 24.0 / 28.0;

#[derive(Clone, Copy, Default, Eq, PartialEq, Hash, Debug, States)]
enum AppState {
    #[default]
    Title,
    InGame,
    End,
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
enum RenderMode {
    Cpu,
    Gpu,
}

impl RenderMode {
    fn is_cpu(self) -> bool {
        matches!(self, RenderMode::Cpu)
    }
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
enum RenderBackend {
    Auto,
    Gl,
    Vulkan,
}

impl RenderBackend {
    fn backends(self) -> Option<Backends> {
        match self {
            RenderBackend::Auto => None,
            RenderBackend::Gl => Some(Backends::GL),
            RenderBackend::Vulkan => Some(Backends::VULKAN),
        }
    }
}

#[derive(Resource)]
struct GameAssets {
    pixel: Handle<Image>,
    floor: Handle<Image>,
    ledge: Handle<Image>,
    wall: Handle<Image>,
    background: Handle<Image>,
    key: Handle<Image>,
    door: Handle<Image>,
    sofia: Handle<Image>,
    spike: Handle<Image>,
    ladder: Handle<Image>,
    torch: Handle<Image>,
    glow: Handle<Image>,
    player: Handle<Image>,
    guard: Handle<Image>,
    slash: Handle<Image>,
}

#[derive(Resource)]
struct AudioAssets {
    ambient: Handle<AudioSource>,
    key: Handle<AudioSource>,
    door: Handle<AudioSource>,
    win: Handle<AudioSource>,
    alert: Handle<AudioSource>,
    jump: Handle<AudioSource>,
}

#[derive(Resource)]
struct UiAssets {
    font: Handle<Font>,
}

#[derive(Resource, Clone, Copy)]
struct LabelSettings {
    enabled: bool,
}

#[derive(Resource)]
struct AtlasAssets {
    player: Handle<TextureAtlasLayout>,
    guard: Handle<TextureAtlasLayout>,
    sofia: Handle<TextureAtlasLayout>,
}

#[derive(Resource, Default)]
struct SessionState {
    has_key: bool,
    hud_key_icon: Option<Entity>,
}

#[derive(Resource)]
struct GuardSpawns(Vec<GuardSpawn>);

#[derive(Clone, Copy)]
struct GuardSpawn {
    pos: Vec2,
    left: f32,
    right: f32,
    label: &'static str,
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct PlayerState {
    speed: f32,
    jump_velocity: f32,
    gravity: f32,
    coyote_time: f32,
    jump_buffer: f32,
    climb_speed: f32,
    coyote_timer: f32,
    jump_buffer_timer: f32,
    on_ladder: bool,
    respawn_position: Vec2,
    walk_timer: f32,
    attack_cooldown: f32,
    attack_active: f32,
    facing: f32,
    slash_entity: Entity,
}

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Component)]
struct Guard {
    speed: f32,
    gravity: f32,
    left_limit: f32,
    right_limit: f32,
    direction: f32,
    walk_timer: f32,
    alive: bool,
}

#[derive(Component)]
struct Collider {
    size: Vec2,
}

#[derive(Component)]
struct Solid;

#[derive(Component)]
struct Hazard;

#[derive(Component)]
struct Ladder;

#[derive(Component)]
struct Key;

#[derive(Component)]
struct Gap;

#[derive(Component)]
struct Door;

#[derive(Component)]
struct DoorBlocker;

#[derive(Component)]
struct Checkpoint;

#[derive(Component)]
struct Princess;

#[derive(Component)]
struct KeyFloat {
    base_y: f32,
    time: f32,
}

#[derive(Component)]
struct PrincessWave {
    time: f32,
}

#[derive(Component)]
struct TorchLight {
    base_color: Color,
    phase: f32,
}

#[derive(Component)]
struct DoorOpening {
    start: f32,
    end: f32,
    timer: Timer,
}

#[derive(Component)]
struct FadeOut {
    timer: Timer,
}

#[derive(Component)]
struct Slash;

#[derive(Message)]
struct RespawnEvent;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct InGameSet;

#[derive(Component)]
struct TitleUi;

#[derive(Component)]
struct EndUi;

#[derive(Component)]
struct GameCamera;

fn main() {
    let render_mode = render_mode_from_args();
    let render_backend = render_backend_from_args();
    let labels_enabled = labels_enabled_from_args();
    let render_plugin = if render_mode.is_cpu() || render_backend != RenderBackend::Auto {
        RenderPlugin {
            render_creation: WgpuSettings {
                power_preference: if render_mode.is_cpu() {
                    PowerPreference::LowPower
                } else {
                    PowerPreference::HighPerformance
                },
                force_fallback_adapter: render_mode.is_cpu(),
                backends: render_backend.backends(),
                ..default()
            }
            .into(),
            ..default()
        }
    } else {
        RenderPlugin::default()
    };

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: "..".to_string(),
                ..default()
            })
            .set(render_plugin)
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Prince of Oliver".to_string(),
                    resolution: (1280, 720).into(),
                    resizable: true,
                    ..default()
                }),
                ..default()
            }),
    );

    app.insert_resource(render_mode)
        .insert_resource(render_backend)
        .insert_resource(LabelSettings {
            enabled: labels_enabled,
        })
        .init_state::<AppState>()
        .init_resource::<SessionState>()
        .add_message::<RespawnEvent>()
        .add_systems(Startup, setup)
        .add_systems(OnEnter(AppState::InGame), spawn_level)
        .add_systems(
            Update,
            (
                title_input.run_if(in_state(AppState::Title)),
                end_input.run_if(in_state(AppState::End)),
                spawn_title_ui.run_if(in_state(AppState::Title)),
                spawn_end_ui.run_if(in_state(AppState::End)),
            ),
        )
        .configure_sets(Update, InGameSet.run_if(in_state(AppState::InGame)))
        .add_systems(
            Update,
            (
                player_system,
                guard_system,
                key_pickup_system,
                checkpoint_system,
                princess_rescue_system,
                hazard_system,
                guard_hit_system,
                respawn_system,
            )
                .in_set(InGameSet),
        )
        .add_systems(
            Update,
            (
                door_open_system,
                fade_out_system,
                animate_key_system,
                animate_princess_system,
                animate_torches_system,
                camera_follow_system,
            )
                .in_set(InGameSet),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut images: ResMut<Assets<Image>>,
    render_mode: Res<RenderMode>,
) {
    if render_mode.is_cpu() {
        let target = create_low_res_target(&mut images);

        commands.spawn((
            Camera2d,
            Camera::default(),
            RenderTarget::Image(target.clone().into()),
            Msaa::Off,
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::Fixed {
                    width: VIEW_WIDTH,
                    height: VIEW_HEIGHT,
                },
                ..OrthographicProjection::default_2d()
            }),
            Transform::from_xyz(VIEW_WIDTH * 0.5, VIEW_HEIGHT * 0.5, 100.0),
            GameCamera,
            IsDefaultUiCamera,
        ));

        commands.spawn((
            Camera2d,
            Camera {
                order: 1,
                ..default()
            },
            Msaa::Off,
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::Fixed {
                    width: VIEW_WIDTH,
                    height: VIEW_HEIGHT,
                },
                ..OrthographicProjection::default_2d()
            }),
            Transform::from_xyz(VIEW_WIDTH * 0.5, VIEW_HEIGHT * 0.5, 100.0),
            RenderLayers::layer(PRESENT_LAYER),
        ));

        commands.spawn((
            Sprite {
                image: target,
                custom_size: Some(Vec2::new(VIEW_WIDTH, VIEW_HEIGHT)),
                ..default()
            },
            Transform::from_xyz(VIEW_WIDTH * 0.5, VIEW_HEIGHT * 0.5, Z_BG - 1.0),
            RenderLayers::layer(PRESENT_LAYER),
        ));
    } else {
        commands.spawn((
            Camera2d,
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::Fixed {
                    width: VIEW_WIDTH,
                    height: VIEW_HEIGHT,
                },
                ..OrthographicProjection::default_2d()
            }),
            Transform::from_xyz(VIEW_WIDTH * 0.5, VIEW_HEIGHT * 0.5, 100.0),
            GameCamera,
            IsDefaultUiCamera,
        ));
    }

    let assets = GameAssets {
        pixel: asset_server.load("assets/pixel.png"),
        floor: asset_server.load("assets/floor.png"),
        ledge: asset_server.load("assets/ledge.png"),
        wall: asset_server.load("assets/wall.png"),
        background: asset_server.load("assets/background.png"),
        key: asset_server.load("assets/key.png"),
        door: asset_server.load("assets/door.png"),
        sofia: asset_server.load("assets/sofia.png"),
        spike: asset_server.load("assets/spike.png"),
        ladder: asset_server.load("assets/ladder.png"),
        torch: asset_server.load("assets/torch.png"),
        glow: asset_server.load("assets/glow.png"),
        player: asset_server.load("assets/player.png"),
        guard: asset_server.load("assets/guard.png"),
        slash: asset_server.load("assets/slash.png"),
    };

    let audio = AudioAssets {
        ambient: asset_server.load("assets/audio/ambient.wav"),
        key: asset_server.load("assets/audio/key.wav"),
        door: asset_server.load("assets/audio/door.wav"),
        win: asset_server.load("assets/audio/win.wav"),
        alert: asset_server.load("assets/audio/alert.wav"),
        jump: asset_server.load("assets/audio/jump.wav"),
    };

    let ui_assets = UiAssets {
        font: asset_server.load("vendor/bevy/assets/fonts/FiraSans-Bold.ttf"),
    };

    let player_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(24, 24),
        2,
        1,
        None,
        None,
    ));
    let guard_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(24, 24),
        2,
        1,
        None,
        None,
    ));
    let sofia_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(24, 28),
        2,
        1,
        None,
        None,
    ));

    commands.insert_resource(assets);
    commands.insert_resource(audio);
    commands.insert_resource(ui_assets);
    commands.insert_resource(AtlasAssets {
        player: player_layout,
        guard: guard_layout,
        sofia: sofia_layout,
    });

    commands.insert_resource(GuardSpawns(vec![GuardSpawn {
        pos: to_world(Vec2::new(620.0, 180.0)),
        left: 540.0,
        right: 700.0,
        label: "guard1",
    }]));
}

fn spawn_title_ui(
    mut commands: Commands,
    ui: Res<UiAssets>,
    existing: Query<Entity, With<TitleUi>>,
) {
    if !existing.is_empty() {
        return;
    }
    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        DespawnOnExit(AppState::Title),
        TitleUi,
        children![spawn_centered_text(
            ui.font.clone(),
            "PRINCE OF OLIVER\n\nOliver descends into the dungeon to rescue Princess Sofia.\nThe gates are locked, the shadows hide a guard...\n\nPress Space to start.",
        )],
    ));
}

fn spawn_end_ui(
    mut commands: Commands,
    ui: Res<UiAssets>,
    existing: Query<Entity, With<EndUi>>,
) {
    if !existing.is_empty() {
        return;
    }
    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        DespawnOnExit(AppState::End),
        EndUi,
        children![spawn_centered_text(
            ui.font.clone(),
            "Sofia is safe.\n\nMade for Oliver & Sofia.\n\nPress Space to return.",
        )],
    ));
}

fn spawn_centered_text(font: Handle<Font>, text: &str) -> impl Bundle {
    (
        Node {
            width: percent(100),
            height: percent(100),
            padding: UiRect::all(px(24.0)),
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![(
            Text::new(text),
            TextFont {
                font,
                font_size: 32.0,
                ..default()
            },
            TextColor(Color::srgba(0.95, 0.92, 0.85, 1.0)),
            LineHeight::RelativeToFont(1.4),
            TextLayout::new_with_justify(Justify::Center),
        )],
    )
}

fn title_input(keys: Res<ButtonInput<KeyCode>>, mut next_state: ResMut<NextState<AppState>>) {
    if keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter) {
        next_state.set(AppState::InGame);
    }
}

fn end_input(keys: Res<ButtonInput<KeyCode>>, mut next_state: ResMut<NextState<AppState>>) {
    if keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter) {
        next_state.set(AppState::Title);
    }
}

fn spawn_level(
    mut commands: Commands,
    assets: Res<GameAssets>,
    audio: Res<AudioAssets>,
    ui: Res<UiAssets>,
    atlases: Res<AtlasAssets>,
    render_mode: Res<RenderMode>,
    labels: Res<LabelSettings>,
    mut session: ResMut<SessionState>,
    guard_spawns: Res<GuardSpawns>,
) {
    session.has_key = false;
    session.hud_key_icon = None;

    spawn_background(&mut commands, &assets, &render_mode);
    spawn_platforms(&mut commands, &assets, &render_mode, &ui, &labels);
    spawn_ladders(&mut commands, &assets, &ui, &labels);
    spawn_spikes(&mut commands, &assets, &ui, &labels);
    spawn_kill_zone(&mut commands, &ui, &labels);
    spawn_key(&mut commands, &assets, &ui, &labels);
    spawn_checkpoint(&mut commands, &ui, &labels);
    spawn_door(&mut commands, &assets, &render_mode, &ui, &labels);
    spawn_princess(&mut commands, &assets, &atlases, &render_mode, &ui, &labels);
    spawn_player(&mut commands, &assets, &atlases, &render_mode, &ui, &labels);
    spawn_guards(&mut commands, &assets, &atlases, &guard_spawns, &ui, &labels);
    spawn_torches(&mut commands, &assets, &render_mode, &ui, &labels);
    spawn_hud(&mut commands, &assets, &ui, &mut session);

    commands.spawn((
        AudioPlayer::new(audio.ambient.clone()),
        PlaybackSettings {
            volume: Volume::Linear(db_to_linear(-12.0)),
            ..PlaybackSettings::LOOP
        },
        DespawnOnExit(AppState::InGame),
    ));
}

fn spawn_background(commands: &mut Commands, assets: &GameAssets, render_mode: &RenderMode) {
    let center = Vec2::new(LEVEL_WIDTH * 0.5, LEVEL_HEIGHT * 0.5);

    commands.spawn((
        Sprite {
            image: assets.background.clone(),
            custom_size: Some(Vec2::new(LEVEL_WIDTH, LEVEL_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(center.x, center.y, Z_BG),
        DespawnOnExit(AppState::InGame),
    ));

    if !render_mode.is_cpu() {
        commands.spawn((
            Sprite {
                image: assets.wall.clone(),
                custom_size: Some(Vec2::new(LEVEL_WIDTH, LEVEL_HEIGHT)),
                color: Color::srgba(0.7, 0.7, 0.75, 0.35),
                ..default()
            },
            Transform::from_xyz(center.x, center.y, Z_WALL),
            DespawnOnExit(AppState::InGame),
        ));

        commands.spawn((
            Sprite {
                image: assets.pixel.clone(),
                custom_size: Some(Vec2::new(LEVEL_WIDTH, LEVEL_HEIGHT)),
                color: Color::srgba(0.0, 0.0, 0.0, 0.2),
                ..default()
            },
            Transform::from_xyz(center.x, center.y, Z_VIGNETTE),
            DespawnOnExit(AppState::InGame),
        ));
    }
}

fn spawn_platforms(
    commands: &mut Commands,
    assets: &GameAssets,
    render_mode: &RenderMode,
    ui: &UiAssets,
    labels: &LabelSettings,
) {
    let floors = [
        ("floor1", Vec2::new(180.0, 210.0), Vec2::new(360.0, 24.0)),
        ("floor2", Vec2::new(600.0, 210.0), Vec2::new(400.0, 24.0)),
        ("floor3", Vec2::new(1010.0, 210.0), Vec2::new(340.0, 24.0)),
        ("floor4", Vec2::new(1390.0, 210.0), Vec2::new(420.0, 24.0)),
    ];

    for (name, pos, size) in floors {
        spawn_platform(
            commands,
            assets,
            name,
            to_world(pos),
            size,
            assets.floor.clone(),
            render_mode,
            ui,
            labels,
        );
    }

    spawn_gap_labels(commands, &floors, ui, labels);

    let ledges = [
        ("ledge1", Vec2::new(240.0, 130.0), Vec2::new(100.0, 16.0)),
        ("ledge2", Vec2::new(360.0, 150.0), Vec2::new(80.0, 16.0)),
        ("ledge3", Vec2::new(500.0, 160.0), Vec2::new(120.0, 16.0)),
        ("ledge4", Vec2::new(880.0, 140.0), Vec2::new(120.0, 16.0)),
    ];

    for (name, pos, size) in ledges {
        spawn_platform(
            commands,
            assets,
            name,
            to_world(pos),
            size,
            assets.ledge.clone(),
            render_mode,
            ui,
            labels,
        );
    }
}

fn spawn_platform(
    commands: &mut Commands,
    assets: &GameAssets,
    name: &str,
    pos: Vec2,
    size: Vec2,
    texture: Handle<Image>,
    render_mode: &RenderMode,
    ui: &UiAssets,
    labels: &LabelSettings,
) {
    let mut sprite = Sprite::from_image(texture);
    sprite.custom_size = Some(size);

    let entity = commands
        .spawn((
            sprite,
            Transform::from_xyz(pos.x, pos.y, Z_PLATFORM),
            Collider { size },
            Solid,
            Name::new(name.to_string()),
            DespawnOnExit(AppState::InGame),
        ))
        .id();

    if !render_mode.is_cpu() {
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Sprite {
                    image: assets.pixel.clone(),
                    custom_size: Some(Vec2::new(size.x, 3.0)),
                    color: Color::srgba(0.9, 0.85, 0.7, 0.35),
                    ..default()
                },
                Transform::from_xyz(0.0, size.y * 0.5 - 2.0, 0.1),
            ));
        });
    }

    maybe_attach_label(
        commands,
        ui,
        labels,
        entity,
        name,
        Vec2::new(0.0, size.y * 0.5 + 8.0),
    );
}

fn spawn_gap_labels(
    commands: &mut Commands,
    floors: &[(&str, Vec2, Vec2)],
    ui: &UiAssets,
    labels: &LabelSettings,
) {
    if !labels.enabled {
        return;
    }

    let mut gap_index = 1;
    for window in floors.windows(2) {
        let (_, left_pos, left_size) = window[0];
        let (_, right_pos, right_size) = window[1];
        let left_edge = left_pos.x + left_size.x * 0.5;
        let right_edge = right_pos.x - right_size.x * 0.5;
        if right_edge <= left_edge {
            continue;
        }

        let center_x = (left_edge + right_edge) * 0.5;
        let center_y = left_pos.y;
        let label = format!("gap{}", gap_index);
        gap_index += 1;
        let world_pos = to_world(Vec2::new(center_x, center_y));
        let entity = commands
            .spawn((
                Transform::from_xyz(world_pos.x, world_pos.y, Z_INTERACT),
                Gap,
                Name::new(label.clone()),
                DespawnOnExit(AppState::InGame),
            ))
            .id();

        maybe_attach_label(
            commands,
            ui,
            labels,
            entity,
            label.as_str(),
            Vec2::new(0.0, 16.0),
        );
    }
}

fn maybe_attach_label(
    commands: &mut Commands,
    ui: &UiAssets,
    labels: &LabelSettings,
    entity: Entity,
    text: &str,
    offset: Vec2,
) {
    if !labels.enabled {
        return;
    }

    commands.entity(entity).with_children(|parent| {
        parent.spawn((
            Text2d::new(text),
            TextFont {
                font: ui.font.clone(),
                font_size: 10.0,
                ..default()
            },
            TextColor(Color::srgba(0.95, 0.9, 0.75, 1.0)),
            TextLayout::new_with_justify(Justify::Center),
            Transform::from_xyz(offset.x, offset.y, Z_LABEL),
        ));
    });
}

fn spawn_ladders(commands: &mut Commands, assets: &GameAssets, ui: &UiAssets, labels: &LabelSettings) {
    spawn_ladder(commands, assets, "ladder1", Vec2::new(180.0, 160.0), 64.0, ui, labels);
    spawn_ladder(commands, assets, "ladder2", Vec2::new(1025.0, 140.0), 102.0, ui, labels);
}

fn spawn_ladder(
    commands: &mut Commands,
    assets: &GameAssets,
    name: &str,
    pos: Vec2,
    height: f32,
    ui: &UiAssets,
    labels: &LabelSettings,
) {
    let pos = to_world(pos);
    let entity = commands
        .spawn((
        Sprite {
            image: assets.ladder.clone(),
            custom_size: Some(Vec2::new(16.0, height)),
            ..default()
        },
        Transform::from_xyz(pos.x, pos.y, Z_INTERACT),
        Collider {
            size: Vec2::new(16.0, height),
        },
        Ladder,
        Name::new(name.to_string()),
        DespawnOnExit(AppState::InGame),
    ))
    .id();

    maybe_attach_label(
        commands,
        ui,
        labels,
        entity,
        name,
        Vec2::new(0.0, height * 0.5 + 6.0),
    );
}

fn spawn_spikes(commands: &mut Commands, assets: &GameAssets, ui: &UiAssets, labels: &LabelSettings) {
    let pos = to_world(Vec2::new(740.0, 198.0));
    let entity = commands
        .spawn((
            Transform::from_xyz(pos.x, pos.y, Z_INTERACT),
            Collider {
                size: Vec2::new(32.0, 14.0),
            },
            Hazard,
            Name::new("spikes1"),
            DespawnOnExit(AppState::InGame),
        ))
        .id();

    commands.entity(entity).with_children(|parent| {
        parent.spawn((
            Sprite {
                image: assets.spike.clone(),
                custom_size: Some(Vec2::new(32.0, 16.0)),
                ..default()
            },
            Transform::from_xyz(0.0, 6.0, 0.1),
        ));
    });

    maybe_attach_label(
        commands,
        ui,
        labels,
        entity,
        "spikes1",
        Vec2::new(0.0, 18.0),
    );
}

fn spawn_kill_zone(commands: &mut Commands, ui: &UiAssets, labels: &LabelSettings) {
    let pos = to_world(Vec2::new(LEVEL_WIDTH * 0.5, LEVEL_HEIGHT + 40.0));
    let entity = commands
        .spawn((
        Transform::from_xyz(pos.x, pos.y, Z_INTERACT),
        Collider {
            size: Vec2::new(LEVEL_WIDTH, 80.0),
        },
        Hazard,
        Name::new("kill_zone1"),
        DespawnOnExit(AppState::InGame),
    ))
    .id();

    maybe_attach_label(
        commands,
        ui,
        labels,
        entity,
        "kill_zone1",
        Vec2::new(0.0, 50.0),
    );
}

fn spawn_key(commands: &mut Commands, assets: &GameAssets, ui: &UiAssets, labels: &LabelSettings) {
    let pos = to_world(Vec2::new(880.0, 126.0));
    let entity = commands
        .spawn((
        Sprite::from_image(assets.key.clone()),
        Transform::from_xyz(pos.x, pos.y, Z_INTERACT),
        Collider {
            size: Vec2::new(12.0, 12.0),
        },
        Key,
        Name::new("key1"),
        KeyFloat {
            base_y: pos.y,
            time: 0.0,
        },
        DespawnOnExit(AppState::InGame),
    ))
    .id();

    maybe_attach_label(
        commands,
        ui,
        labels,
        entity,
        "key1",
        Vec2::new(0.0, 14.0),
    );
}

fn spawn_checkpoint(commands: &mut Commands, ui: &UiAssets, labels: &LabelSettings) {
    let pos = to_world(Vec2::new(990.0, 190.0));
    let entity = commands
        .spawn((
        Transform::from_xyz(pos.x, pos.y, Z_INTERACT),
        Collider {
            size: Vec2::new(20.0, 20.0),
        },
        Checkpoint,
        Name::new("checkpoint1"),
        DespawnOnExit(AppState::InGame),
    ))
    .id();

    maybe_attach_label(
        commands,
        ui,
        labels,
        entity,
        "checkpoint1",
        Vec2::new(0.0, 18.0),
    );
}

fn spawn_door(
    commands: &mut Commands,
    assets: &GameAssets,
    render_mode: &RenderMode,
    ui: &UiAssets,
    labels: &LabelSettings,
) {
    let pos = to_world(Vec2::new(1230.0, 170.0));
    let door_entity = commands
        .spawn((
        Sprite {
            image: assets.door.clone(),
            color: Color::srgba(0.55, 0.45, 0.3, 1.0),
            ..default()
        },
        Transform::from_xyz(pos.x, pos.y, Z_INTERACT),
        Door,
        Name::new("door1"),
        DespawnOnExit(AppState::InGame),
    ))
    .id();

    commands.spawn((
        Transform::from_xyz(pos.x, pos.y, Z_INTERACT),
        Collider {
            size: Vec2::new(24.0, 60.0),
        },
        Solid,
        DoorBlocker,
        Name::new("door_blocker1"),
        DespawnOnExit(AppState::InGame),
    ));

    if !render_mode.is_cpu() {
        let lintel_pos = to_world(Vec2::new(1230.0, 130.0));
        commands.spawn((
            Sprite {
                image: assets.wall.clone(),
                custom_size: Some(Vec2::new(60.0, 12.0)),
                color: Color::srgba(0.5, 0.5, 0.55, 0.6),
                ..default()
            },
            Transform::from_xyz(lintel_pos.x, lintel_pos.y, Z_PLATFORM - 1.0),
            DespawnOnExit(AppState::InGame),
        ));
    }

    maybe_attach_label(
        commands,
        ui,
        labels,
        door_entity,
        "door1",
        Vec2::new(0.0, 36.0),
    );
}

fn spawn_princess(
    commands: &mut Commands,
    assets: &GameAssets,
    atlases: &AtlasAssets,
    render_mode: &RenderMode,
    ui: &UiAssets,
    labels: &LabelSettings,
) {
    let pos = to_world(Vec2::new(1500.0, 181.0));
    let entity = commands
        .spawn((
            Transform::from_xyz(pos.x, pos.y, Z_INTERACT),
            Collider {
                size: Vec2::new(18.0, 26.0),
            },
            Princess,
            Name::new("princess1"),
            DespawnOnExit(AppState::InGame),
        ))
        .id();

    commands.entity(entity).with_children(|parent| {
        if !render_mode.is_cpu() {
            parent.spawn((
                Sprite {
                    image: assets.pixel.clone(),
                    custom_size: Some(Vec2::new(10.0, 3.0)),
                    color: Color::srgba(0.0, 0.0, 0.0, 0.35),
                    ..default()
                },
                Transform::from_xyz(0.0, -12.0, -0.1),
            ));
        }

        parent.spawn((
            Sprite::from_atlas_image(
                assets.sofia.clone(),
                TextureAtlas {
                    layout: atlases.sofia.clone(),
                    index: 0,
                },
            ),
            Transform {
                translation: Vec3::new(0.0, 0.0, 0.1),
                scale: Vec3::splat(PRINCESS_SCALE),
                ..default()
            },
            PrincessWave { time: 0.0 },
        ));
    });

    maybe_attach_label(
        commands,
        ui,
        labels,
        entity,
        "princess1",
        Vec2::new(0.0, 20.0),
    );
}

fn spawn_player(
    commands: &mut Commands,
    assets: &GameAssets,
    atlases: &AtlasAssets,
    render_mode: &RenderMode,
    ui: &UiAssets,
    labels: &LabelSettings,
) {
    let pos = to_world(Vec2::new(80.0, 180.0));
    let player_entity = commands
        .spawn((
            Sprite::from_atlas_image(
                assets.player.clone(),
                TextureAtlas {
                    layout: atlases.player.clone(),
                    index: 0,
                },
            ),
            Transform::from_xyz(pos.x, pos.y, Z_ACTOR),
            Collider {
                size: Vec2::new(14.0, 24.0),
            },
            Velocity(Vec2::ZERO),
            Player,
            Name::new("player1"),
            DespawnOnExit(AppState::InGame),
        ))
        .id();

    let mut slash_entity = Entity::PLACEHOLDER;
    commands.entity(player_entity).with_children(|parent| {
        if !render_mode.is_cpu() {
            parent.spawn((
                Sprite {
                    image: assets.pixel.clone(),
                    custom_size: Some(Vec2::new(12.0, 3.0)),
                    color: Color::srgba(0.0, 0.0, 0.0, 0.35),
                    ..default()
                },
                Transform::from_xyz(0.0, -12.0, -0.1),
            ));
        }

        slash_entity = parent
            .spawn((
                Sprite {
                    image: assets.slash.clone(),
                    ..default()
                },
                Transform {
                    translation: Vec3::new(14.0, 4.0, 0.2),
                    scale: Vec3::splat(0.66),
                    ..default()
                },
                Visibility::Hidden,
                Slash,
            ))
            .id();
    });

    commands.entity(player_entity).insert(PlayerState {
        speed: 90.0,
        jump_velocity: 190.0,
        gravity: -520.0,
        coyote_time: 0.12,
        jump_buffer: 0.12,
        climb_speed: 60.0,
        coyote_timer: 0.12,
        jump_buffer_timer: 0.0,
        on_ladder: false,
        respawn_position: pos,
        walk_timer: 0.0,
        attack_cooldown: 0.0,
        attack_active: 0.0,
        facing: 1.0,
        slash_entity,
    });

    maybe_attach_label(
        commands,
        ui,
        labels,
        player_entity,
        "player1",
        Vec2::new(0.0, 20.0),
    );

}

fn spawn_guards(
    commands: &mut Commands,
    assets: &GameAssets,
    atlases: &AtlasAssets,
    guard_spawns: &GuardSpawns,
    ui: &UiAssets,
    labels: &LabelSettings,
) {
    for spawn in guard_spawns.0.iter().copied() {
        let entity = commands
            .spawn((
            Sprite::from_atlas_image(
                assets.guard.clone(),
                TextureAtlas {
                    layout: atlases.guard.clone(),
                    index: 0,
                },
            ),
            Transform::from_xyz(spawn.pos.x, spawn.pos.y, Z_ACTOR - 1.0),
            Collider {
                size: Vec2::new(14.0, 24.0),
            },
            Velocity(Vec2::ZERO),
            Guard {
                speed: 40.0,
                gravity: -520.0,
                left_limit: spawn.left,
                right_limit: spawn.right,
                direction: 1.0,
                walk_timer: 0.0,
                alive: true,
            },
            Name::new(spawn.label),
            DespawnOnExit(AppState::InGame),
        ))
        .id();

        maybe_attach_label(
            commands,
            ui,
            labels,
            entity,
            spawn.label,
            Vec2::new(0.0, 20.0),
        );
    }
}

fn spawn_torches(
    commands: &mut Commands,
    assets: &GameAssets,
    render_mode: &RenderMode,
    ui: &UiAssets,
    labels: &LabelSettings,
) {
    let torches = [
        ("torch1", Vec2::new(140.0, 150.0)),
        ("torch2", Vec2::new(620.0, 150.0)),
        ("torch3", Vec2::new(980.0, 150.0)),
        ("torch4", Vec2::new(1320.0, 150.0)),
    ];

    for (index, (name, pos)) in torches.iter().enumerate() {
        let pos = to_world(*pos);
        let entity = commands
            .spawn((
                Sprite::from_image(assets.torch.clone()),
                Transform::from_xyz(pos.x, pos.y, Z_INTERACT),
                Name::new(*name),
                DespawnOnExit(AppState::InGame),
            ))
            .id();

        if !render_mode.is_cpu() {
            let glow_color = Color::srgba(1.0, 0.8, 0.55, TORCH_GLOW_BASE_ALPHA);
            commands.spawn((
                Sprite {
                    image: assets.glow.clone(),
                    color: glow_color,
                    ..default()
                },
                Transform {
                    translation: Vec3::new(pos.x, pos.y + TORCH_GLOW_Y_OFFSET, Z_GLOW),
                    scale: Vec3::splat(0.6),
                    ..default()
                },
                TorchLight {
                    base_color: glow_color,
                    phase: index as f32 * 1.7,
                },
                DespawnOnExit(AppState::InGame),
            ));
        }

        maybe_attach_label(
            commands,
            ui,
            labels,
            entity,
            name,
            Vec2::new(0.0, 14.0),
        );
    }
}

fn spawn_hud(
    commands: &mut Commands,
    assets: &GameAssets,
    ui: &UiAssets,
    session: &mut SessionState,
) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(8.0),
                top: px(8.0),
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6.0),
                ..default()
            },
            DespawnOnExit(AppState::InGame),
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        let icon = parent
            .spawn((
                ImageNode::new(assets.key.clone()).with_color(Color::srgba(0.5, 0.5, 0.5, 0.8)),
                Node {
                    width: px(12.0),
                    height: px(12.0),
                    ..default()
                },
            ))
            .id();
        session.hud_key_icon = Some(icon);

        parent.spawn((
            Text::new("Key"),
            TextFont {
                font: ui.font.clone(),
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgba(0.9, 0.85, 0.75, 0.9)),
        ));
    });
}

fn player_system(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    audio: Res<AudioAssets>,
    mut commands: Commands,
    solids: Query<(&Transform, &Collider), (With<Solid>, Without<Player>)>,
    ladders: Query<(&Transform, &Collider), (With<Ladder>, Without<Player>)>,
    mut player_q: Query<
        (&mut Transform, &mut Sprite, &mut PlayerState, &mut Velocity, &Collider),
        With<Player>,
    >,
    mut guards: Query<
        (Entity, &Transform, &mut Guard, &Collider, &mut Velocity),
        (With<Guard>, Without<Player>),
    >,
    mut slash_q: Query<
        (&mut Transform, &mut Sprite, &mut Visibility),
        (With<Slash>, Without<Player>, Without<Guard>, Without<Solid>, Without<Ladder>),
    >,
) {
    let dt = time.delta_secs();
    if let Ok((mut transform, mut sprite, mut state, mut velocity, collider)) =
        player_q.single_mut()
    {
        let pos = Vec2::new(transform.translation.x, transform.translation.y);
        let set_respawn = keys.just_pressed(KeyCode::KeyR);
        let ladder_hit = ladders.iter().any(|(ladder_tf, ladder_collider)| {
            aabb_intersects(
                pos,
                collider.size,
                Vec2::new(ladder_tf.translation.x, ladder_tf.translation.y),
                ladder_collider.size,
            )
        });
        state.on_ladder = ladder_hit;

        state.attack_cooldown = (state.attack_cooldown - dt).max(0.0);
        if state.attack_active > 0.0 {
            state.attack_active = (state.attack_active - dt).max(0.0);
            if state.attack_active == 0.0 {
                if let Ok((_, _, mut visibility)) = slash_q.get_mut(state.slash_entity) {
                    *visibility = Visibility::Hidden;
                }
            }
        }

        let on_ground = state.coyote_timer > 0.0;
        let input_dir = move_input(&keys);
        let crouching =
            (keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS)) && on_ground;
        let mut max_speed = state.speed;
        if crouching {
            max_speed *= 0.4;
        }

        if keys.just_pressed(KeyCode::KeyE) && state.attack_cooldown == 0.0 {
            state.attack_cooldown = 0.35;
            state.attack_active = 0.18;
            if let Ok((_, _, mut visibility)) = slash_q.get_mut(state.slash_entity) {
                *visibility = Visibility::Visible;
            }
            try_hit_guard(pos, state.facing, &mut guards, &mut commands);
        }

        if state.on_ladder {
            let climb_dir = climb_input(&keys);
            velocity.x = input_dir * state.speed * 0.6;
            velocity.y = climb_dir * state.climb_speed;
            if keys.just_pressed(KeyCode::Space) {
                state.on_ladder = false;
                velocity.y = state.jump_velocity;
                play_sfx(&mut commands, audio.jump.clone(), db_to_linear(-6.0));
            }
        } else {
            velocity.x = input_dir * max_speed;
            state.coyote_timer = (state.coyote_timer - dt).max(0.0);
            if keys.just_pressed(KeyCode::Space) {
                state.jump_buffer_timer = state.jump_buffer;
            } else {
                state.jump_buffer_timer = (state.jump_buffer_timer - dt).max(0.0);
            }

            if state.jump_buffer_timer > 0.0 && state.coyote_timer > 0.0 {
                velocity.y = state.jump_velocity;
                state.jump_buffer_timer = 0.0;
                state.coyote_timer = 0.0;
                play_sfx(&mut commands, audio.jump.clone(), db_to_linear(-6.0));
            }

            velocity.y += state.gravity * dt;
        }

        let mut new_pos = pos;
        let delta = **velocity * dt;
        let (hit_x, hit_y) = move_with_collisions(&mut new_pos, delta, collider.size, &solids);
        if hit_x {
            velocity.x = 0.0;
        }
        if hit_y {
            velocity.y = 0.0;
        }
        let on_floor = hit_y && delta.y < 0.0;
        if on_floor {
            state.coyote_timer = state.coyote_time;
        }

        transform.translation.x = new_pos.x;
        transform.translation.y = new_pos.y;
        if set_respawn {
            state.respawn_position = new_pos;
        }

        if input_dir.abs() > 0.1 {
            state.facing = input_dir.signum();
            sprite.flip_x = state.facing < 0.0;
        }

        let moving = velocity.x.abs() > 1.0 && on_floor && !state.on_ladder;
        if moving {
            state.walk_timer += dt * 8.0;
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = state.walk_timer as usize % 2;
            }
        } else if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = 0;
        }

        let dir = if sprite.flip_x { -1.0 } else { 1.0 };
        if let Ok((mut slash_tf, mut slash_sprite, _)) = slash_q.get_mut(state.slash_entity) {
            slash_tf.translation.x = 14.0 * dir;
            slash_tf.translation.y = 4.0;
            slash_sprite.flip_x = sprite.flip_x;
        }

        if state.attack_active > 0.0 {
            try_hit_guard(new_pos, dir, &mut guards, &mut commands);
        }
    }
}

fn guard_system(
    time: Res<Time>,
    solids: Query<(&Transform, &Collider), (With<Solid>, Without<Guard>)>,
    mut guards: Query<
        (&mut Transform, &mut Sprite, &mut Guard, &mut Velocity, &Collider),
        With<Guard>,
    >,
) {
    let dt = time.delta_secs();
    for (mut transform, mut sprite, mut guard, mut velocity, collider) in guards.iter_mut() {
        if !guard.alive {
            continue;
        }

        velocity.x = guard.speed * guard.direction;
        velocity.y += guard.gravity * dt;

        let mut pos = Vec2::new(transform.translation.x, transform.translation.y);
        let delta = **velocity * dt;
        let (hit_x, hit_y) = move_with_collisions(&mut pos, delta, collider.size, &solids);
        if hit_x {
            velocity.x = 0.0;
        }
        if hit_y {
            velocity.y = 0.0;
        }

        if pos.x <= guard.left_limit {
            pos.x = guard.left_limit;
            guard.direction = 1.0;
        } else if pos.x >= guard.right_limit {
            pos.x = guard.right_limit;
            guard.direction = -1.0;
        }

        transform.translation.x = pos.x;
        transform.translation.y = pos.y;

        sprite.flip_x = guard.direction < 0.0;
        guard.walk_timer += dt * 6.0;
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = guard.walk_timer as usize % 2;
        }
    }
}

fn key_pickup_system(
    mut commands: Commands,
    mut session: ResMut<SessionState>,
    audio: Res<AudioAssets>,
    player_q: Query<(&Transform, &Collider), With<Player>>,
    key_q: Query<(Entity, &Transform, &Collider), With<Key>>,
    door_q: Query<(Entity, &Transform), With<Door>>,
    door_blockers: Query<Entity, With<DoorBlocker>>,
    mut hud_icons: Query<&mut ImageNode>,
) {
    if session.has_key {
        return;
    }

    let Ok((player_tf, player_collider)) = player_q.single() else {
        return;
    };
    let player_pos = Vec2::new(player_tf.translation.x, player_tf.translation.y);

    for (key_entity, key_tf, key_collider) in key_q.iter() {
        let key_pos = Vec2::new(key_tf.translation.x, key_tf.translation.y);
        if aabb_intersects(player_pos, player_collider.size, key_pos, key_collider.size) {
            session.has_key = true;
            commands.entity(key_entity).despawn();
            play_sfx(&mut commands, audio.key.clone(), 1.0);

            for blocker in door_blockers.iter() {
                commands.entity(blocker).despawn();
            }
            if let Ok((door_entity, door_tf)) = door_q.single() {
                commands.entity(door_entity).insert(DoorOpening {
                    start: door_tf.translation.y,
                    end: door_tf.translation.y + DOOR_OPEN_OFFSET,
                    timer: Timer::from_seconds(0.35, TimerMode::Once),
                });
                play_sfx(&mut commands, audio.door.clone(), 1.0);
            }

            if let Some(icon_entity) = session.hud_key_icon {
                if let Ok(mut icon) = hud_icons.get_mut(icon_entity) {
                    icon.color = Color::WHITE;
                }
            }

            break;
        }
    }
}

fn checkpoint_system(
    mut player_q: Query<(&Transform, &Collider, &mut PlayerState), With<Player>>,
    checkpoint_q: Query<(&Transform, &Collider), With<Checkpoint>>,
) {
    let Ok((player_tf, player_collider, mut state)) = player_q.single_mut() else {
        return;
    };
    let player_pos = Vec2::new(player_tf.translation.x, player_tf.translation.y);

    for (checkpoint_tf, checkpoint_collider) in checkpoint_q.iter() {
        let pos = Vec2::new(checkpoint_tf.translation.x, checkpoint_tf.translation.y);
        if aabb_intersects(player_pos, player_collider.size, pos, checkpoint_collider.size) {
            state.respawn_position = to_world(Vec2::new(990.0, 170.0));
        }
    }
}

fn princess_rescue_system(
    mut next_state: ResMut<NextState<AppState>>,
    audio: Res<AudioAssets>,
    player_q: Query<(&Transform, &Collider), With<Player>>,
    princess_q: Query<(&Transform, &Collider), With<Princess>>,
    mut commands: Commands,
) {
    let Ok((player_tf, player_collider)) = player_q.single() else {
        return;
    };
    let player_pos = Vec2::new(player_tf.translation.x, player_tf.translation.y);

    for (princess_tf, princess_collider) in princess_q.iter() {
        let pos = Vec2::new(princess_tf.translation.x, princess_tf.translation.y);
        if aabb_intersects(player_pos, player_collider.size, pos, princess_collider.size) {
            play_sfx(&mut commands, audio.win.clone(), 1.0);
            next_state.set(AppState::End);
            break;
        }
    }
}

fn hazard_system(
    mut respawn_writer: MessageWriter<RespawnEvent>,
    audio: Res<AudioAssets>,
    player_q: Query<(&Transform, &Collider), With<Player>>,
    hazard_q: Query<(&Transform, &Collider), With<Hazard>>,
    mut commands: Commands,
) {
    let Ok((player_tf, player_collider)) = player_q.single() else {
        return;
    };
    let player_pos = Vec2::new(player_tf.translation.x, player_tf.translation.y);

    for (hazard_tf, hazard_collider) in hazard_q.iter() {
        let pos = Vec2::new(hazard_tf.translation.x, hazard_tf.translation.y);
        if aabb_intersects(player_pos, player_collider.size, pos, hazard_collider.size) {
            respawn_writer.write(RespawnEvent);
            play_sfx(&mut commands, audio.alert.clone(), 1.0);
            break;
        }
    }
}

fn guard_hit_system(
    mut respawn_writer: MessageWriter<RespawnEvent>,
    audio: Res<AudioAssets>,
    player_q: Query<(&Transform, &Collider), With<Player>>,
    guards: Query<(&Transform, &Collider, &Guard)>,
    mut commands: Commands,
) {
    let Ok((player_tf, player_collider)) = player_q.single() else {
        return;
    };
    let player_pos = Vec2::new(player_tf.translation.x, player_tf.translation.y);

    for (guard_tf, guard_collider, guard) in guards.iter() {
        if !guard.alive {
            continue;
        }
        let guard_pos = Vec2::new(guard_tf.translation.x, guard_tf.translation.y);
        if aabb_intersects(player_pos, player_collider.size, guard_pos, guard_collider.size) {
            respawn_writer.write(RespawnEvent);
            play_sfx(&mut commands, audio.alert.clone(), 1.0);
            break;
        }
    }
}

fn respawn_system(
    mut reader: MessageReader<RespawnEvent>,
    mut player_q: Query<(&mut Transform, &mut Velocity, &mut PlayerState), With<Player>>,
    mut guards: Query<Entity, With<Guard>>,
    mut commands: Commands,
    assets: Res<GameAssets>,
    atlases: Res<AtlasAssets>,
    guard_spawns: Res<GuardSpawns>,
    ui: Res<UiAssets>,
    labels: Res<LabelSettings>,
    mut slash_q: Query<&mut Visibility, (With<Slash>, Without<Player>)>,
) {
    if reader.read().next().is_none() {
        return;
    }

    if let Ok((mut transform, mut velocity, mut state)) = player_q.single_mut() {
        transform.translation.x = state.respawn_position.x;
        transform.translation.y = state.respawn_position.y;
        *velocity = Velocity(Vec2::ZERO);
        state.on_ladder = false;
        state.coyote_timer = state.coyote_time;
        state.jump_buffer_timer = 0.0;
        state.attack_active = 0.0;
        state.attack_cooldown = 0.0;
        if let Ok(mut visibility) = slash_q.get_mut(state.slash_entity) {
            *visibility = Visibility::Hidden;
        }
    }

    for entity in guards.iter_mut() {
        commands.entity(entity).despawn();
    }

    spawn_guards(&mut commands, &assets, &atlases, &guard_spawns, &ui, &labels);
}

fn door_open_system(
    time: Res<Time>,
    mut commands: Commands,
    mut doors: Query<(Entity, &mut Transform, &mut DoorOpening)>,
) {
    let dt = time.delta();
    for (entity, mut transform, mut opening) in doors.iter_mut() {
        opening.timer.tick(dt);
        let t = opening.timer.elapsed_secs() / opening.timer.duration().as_secs_f32();
        let clamped = t.clamp(0.0, 1.0);
        transform.translation.y = opening.start + (opening.end - opening.start) * clamped;
        if opening.timer.is_finished() {
            commands.entity(entity).remove::<DoorOpening>();
        }
    }
}

fn fade_out_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Sprite, &mut FadeOut)>,
) {
    let dt = time.delta();
    for (entity, mut sprite, mut fade) in query.iter_mut() {
        fade.timer.tick(dt);
        let t = fade.timer.elapsed_secs() / fade.timer.duration().as_secs_f32();
        let alpha = (1.0 - t).clamp(0.0, 1.0);
        sprite.color.set_alpha(alpha);
        if fade.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn animate_key_system(time: Res<Time>, mut keys: Query<(&mut Transform, &mut KeyFloat)>) {
    let dt = time.delta_secs();
    for (mut transform, mut float) in keys.iter_mut() {
        float.time += dt;
        transform.translation.y = float.base_y + (float.time * 3.0).sin() * 2.0;
    }
}

fn animate_princess_system(time: Res<Time>, mut sprites: Query<(&mut Sprite, &mut PrincessWave)>) {
    let dt = time.delta_secs();
    for (mut sprite, mut wave) in sprites.iter_mut() {
        wave.time += dt;
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = ((wave.time * 2.0) as usize) % 2;
        }
    }
}

fn animate_torches_system(time: Res<Time>, mut torches: Query<(&mut Sprite, &TorchLight)>) {
    let t = time.elapsed_secs();
    for (mut sprite, torch) in torches.iter_mut() {
        let flicker = 0.04 * (t * 3.5 + torch.phase).sin();
        let mut color = torch.base_color;
        let min_alpha = (torch.base_color.alpha() * TORCH_GLOW_MIN_ALPHA_FACTOR).min(1.0);
        color.set_alpha((torch.base_color.alpha() + flicker).clamp(min_alpha, 1.0));
        sprite.color = color;
    }
}

fn camera_follow_system(
    time: Res<Time>,
    player_q: Query<&Transform, With<Player>>,
    mut camera_q: Query<&mut Transform, (With<GameCamera>, Without<Player>)>,
) {
    let Ok(player_tf) = player_q.single() else {
        return;
    };
    let Ok(mut camera_tf) = camera_q.single_mut() else {
        return;
    };

    let mut target = Vec2::new(player_tf.translation.x, player_tf.translation.y);
    let half_w = VIEW_WIDTH * 0.5;
    let half_h = VIEW_HEIGHT * 0.5;
    target.x = target.x.clamp(half_w, LEVEL_WIDTH - half_w);
    target.y = target.y.clamp(half_h, LEVEL_HEIGHT - half_h);

    let current = Vec2::new(camera_tf.translation.x, camera_tf.translation.y);
    let t = 1.0 - (-6.0_f32 * time.delta_secs()).exp();
    let lerped = current.lerp(target, t);
    camera_tf.translation.x = lerped.x;
    camera_tf.translation.y = lerped.y;
}

fn create_low_res_target(images: &mut Assets<Image>) -> Handle<Image> {
    let size = Extent3d {
        width: VIEW_WIDTH as u32,
        height: VIEW_HEIGHT as u32,
        depth_or_array_layers: 1,
    };
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    image.sampler = ImageSampler::nearest();
    images.add(image)
}

fn render_mode_from_args() -> RenderMode {
    let mut mode = RenderMode::Cpu;
    if let Ok(value) = std::env::var("POO_RENDER_MODE") {
        let value = value.to_lowercase();
        if value == "gpu" || value == "high" || value == "hardware" {
            mode = RenderMode::Gpu;
        } else if value == "cpu" || value == "low" || value == "software" {
            mode = RenderMode::Cpu;
        }
    }

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--gpu" | "--render=gpu" => mode = RenderMode::Gpu,
            "--cpu" | "--render=cpu" => mode = RenderMode::Cpu,
            _ => {}
        }
    }

    mode
}

fn render_backend_from_args() -> RenderBackend {
    let mut backend = RenderBackend::Gl;
    if let Ok(value) = std::env::var("POO_WGPU_BACKEND") {
        let value = value.to_lowercase();
        match value.as_str() {
            "auto" => backend = RenderBackend::Auto,
            "gl" | "opengl" | "opengl3" => backend = RenderBackend::Gl,
            "vk" | "vulkan" => backend = RenderBackend::Vulkan,
            _ => {}
        }
    }

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--backend=auto" => backend = RenderBackend::Auto,
            "--backend=gl" | "--backend=opengl" | "--backend=opengl3" => {
                backend = RenderBackend::Gl
            }
            "--backend=vk" | "--backend=vulkan" => backend = RenderBackend::Vulkan,
            _ => {}
        }
    }

    backend
}

fn labels_enabled_from_args() -> bool {
    for arg in std::env::args().skip(1) {
        if arg == "--labels" {
            return true;
        }
    }
    false
}

fn try_hit_guard<F: QueryFilter>(
    player_pos: Vec2,
    dir: f32,
    guards: &mut Query<(Entity, &Transform, &mut Guard, &Collider, &mut Velocity), F>,
    commands: &mut Commands,
) {
    let sword_pos = player_pos + Vec2::new(12.0 * dir, -4.0);
    let sword_size = Vec2::new(18.0, 10.0);

    for (entity, guard_tf, mut guard, collider, mut velocity) in guards.iter_mut() {
        if !guard.alive {
            continue;
        }
        let guard_pos = Vec2::new(guard_tf.translation.x, guard_tf.translation.y);
        if aabb_intersects(sword_pos, sword_size, guard_pos, collider.size)
            || guard_pos.distance(player_pos) <= 26.0
        {
            guard.alive = false;
            *velocity = Velocity(Vec2::ZERO);
            commands.entity(entity).remove::<Collider>();
            commands.entity(entity).insert(FadeOut {
                timer: Timer::from_seconds(0.3, TimerMode::Once),
            });
        }
    }
}

fn play_sfx(commands: &mut Commands, audio: Handle<AudioSource>, volume: f32) {
    commands.spawn((
        AudioPlayer::new(audio),
        PlaybackSettings {
            volume: Volume::Linear(volume),
            ..PlaybackSettings::DESPAWN
        },
    ));
}

fn move_input(keys: &ButtonInput<KeyCode>) -> f32 {
    let mut dir = 0.0;
    if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
        dir -= 1.0;
    }
    if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
        dir += 1.0;
    }
    dir
}

fn climb_input(keys: &ButtonInput<KeyCode>) -> f32 {
    let mut dir = 0.0;
    if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW) {
        dir += 1.0;
    }
    if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS) {
        dir -= 1.0;
    }
    dir
}

fn aabb_intersects(pos_a: Vec2, size_a: Vec2, pos_b: Vec2, size_b: Vec2) -> bool {
    let half_a = size_a * 0.5;
    let half_b = size_b * 0.5;
    (pos_a.x - pos_b.x).abs() < (half_a.x + half_b.x)
        && (pos_a.y - pos_b.y).abs() < (half_a.y + half_b.y)
}

fn move_with_collisions<F: QueryFilter>(
    pos: &mut Vec2,
    delta: Vec2,
    size: Vec2,
    solids: &Query<(&Transform, &Collider), F>,
) -> (bool, bool) {
    let mut hit_x = false;
    let mut hit_y = false;

    if delta.x != 0.0 {
        pos.x += delta.x;
        let half = size * 0.5;
        for (solid_tf, solid_collider) in solids.iter() {
            let other_pos = Vec2::new(solid_tf.translation.x, solid_tf.translation.y);
            let other_half = solid_collider.size * 0.5;
            if (pos.y - other_pos.y).abs() < (half.y + other_half.y) {
                let min_x = pos.x - half.x;
                let max_x = pos.x + half.x;
                let other_min = other_pos.x - other_half.x;
                let other_max = other_pos.x + other_half.x;
                if max_x > other_min && min_x < other_max {
                    hit_x = true;
                    if delta.x > 0.0 {
                        pos.x = other_min - half.x;
                    } else {
                        pos.x = other_max + half.x;
                    }
                }
            }
        }
    }

    if delta.y != 0.0 {
        pos.y += delta.y;
        let half = size * 0.5;
        for (solid_tf, solid_collider) in solids.iter() {
            let other_pos = Vec2::new(solid_tf.translation.x, solid_tf.translation.y);
            let other_half = solid_collider.size * 0.5;
            if (pos.x - other_pos.x).abs() < (half.x + other_half.x) {
                let min_y = pos.y - half.y;
                let max_y = pos.y + half.y;
                let other_min = other_pos.y - other_half.y;
                let other_max = other_pos.y + other_half.y;
                if max_y > other_min && min_y < other_max {
                    hit_y = true;
                    if delta.y > 0.0 {
                        pos.y = other_min - half.y;
                    } else {
                        pos.y = other_max + half.y;
                    }
                }
            }
        }
    }

    (hit_x, hit_y)
}

fn to_world(pos: Vec2) -> Vec2 {
    Vec2::new(pos.x, LEVEL_HEIGHT - pos.y)
}

fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}
