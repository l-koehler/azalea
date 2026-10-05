use azalea_client::{SprintDirection, StartSprintEvent, test_utils::prelude::*};
use azalea_core::position::{BlockPos, ChunkPos, Vec3};
use azalea_entity::{
    LookDirection, Pose, Position,
    metadata::{Sprinting, Swimming},
};
use azalea_protocol::{
    common::movements::{PositionMoveRotation, RelativeMovements},
    packets::{
        ConnectionProtocol,
        game::{ClientboundBlockUpdate, ClientboundPlayerPosition},
    },
};
use azalea_registry::builtin::BlockKind;

/// Creates a simulation with a pool of water around (0.5, 120, 0.5), with the
/// water surface at `water_top_y`.
fn simulation_in_pool(water_top_y: i32) -> Simulation {
    let mut simulation = Simulation::new(ConnectionProtocol::Game);

    simulation.receive_packet(default_login_packet());
    simulation.tick();

    simulation.receive_packet(make_basic_empty_chunk(ChunkPos::new(0, 0), (384 + 64) / 16));
    for x in 0..6 {
        for z in 0..6 {
            simulation.receive_packet(ClientboundBlockUpdate {
                pos: BlockPos::new(x, 110, z),
                block_state: BlockKind::Stone.into(),
            });
            for y in 111..=water_top_y {
                simulation.receive_packet(ClientboundBlockUpdate {
                    pos: BlockPos::new(x, y, z),
                    block_state: BlockKind::Water.into(),
                });
            }
        }
    }
    simulation.receive_packet(ClientboundPlayerPosition {
        id: 1,
        change: PositionMoveRotation {
            pos: Vec3::new(2.5, 115., 2.5),
            delta: Vec3::ZERO,
            look_direction: LookDirection::default(),
        },
        relative: RelativeMovements::all_absolute(),
    });
    for _ in 0..3 {
        simulation.tick();
    }
    simulation
}

fn start_sprinting(simulation: &mut Simulation) {
    simulation.write_message(StartSprintEvent {
        entity: simulation.entity,
        direction: SprintDirection::Forward,
    });
}

#[test]
fn test_sprinting_underwater_starts_swimming() {
    let _lock = init();

    let mut simulation = simulation_in_pool(125);
    assert!(!*simulation.component::<Swimming>());

    start_sprinting(&mut simulation);
    simulation.tick();
    simulation.tick();

    assert!(*simulation.component::<Sprinting>());
    assert!(*simulation.component::<Swimming>());
    assert_eq!(simulation.component::<Pose>(), Pose::Swimming);
}

#[test]
fn test_swimming_stops_after_leaving_the_water() {
    let _lock = init();

    let mut simulation = simulation_in_pool(125);
    start_sprinting(&mut simulation);
    simulation.tick();
    simulation.tick();
    assert!(*simulation.component::<Swimming>());

    // teleport into the air above the pool
    simulation.receive_packet(ClientboundPlayerPosition {
        id: 2,
        change: PositionMoveRotation {
            pos: Vec3::new(2.5, 140., 2.5),
            delta: Vec3::ZERO,
            look_direction: LookDirection::default(),
        },
        relative: RelativeMovements::all_absolute(),
    });
    for _ in 0..3 {
        simulation.tick();
    }

    // the player is still sprinting (they keep moving forward), but not swimming
    assert!(*simulation.component::<Sprinting>());
    assert!(!*simulation.component::<Swimming>());
    assert_eq!(simulation.component::<Pose>(), Pose::Standing);
}

#[test]
fn test_sprinting_at_the_surface_does_not_start_swimming() {
    let _lock = init();

    // only the player's feet are in water, their eyes are above it
    let mut simulation = simulation_in_pool(115);
    start_sprinting(&mut simulation);
    simulation.tick();
    simulation.tick();

    assert!(!*simulation.component::<Swimming>());
    assert_ne!(simulation.component::<Pose>(), Pose::Swimming);
}

/// Starts swimming, looks in the given direction and returns the player's Y
/// position after a few ticks.
fn swim_with_pitch(pitch: f32) -> f64 {
    let mut simulation = simulation_in_pool(125);
    start_sprinting(&mut simulation);
    simulation.tick();
    simulation.tick();
    assert!(*simulation.component::<Swimming>());

    simulation.with_component_mut::<LookDirection>(|look| *look = LookDirection::new(0., pitch));
    for _ in 0..10 {
        simulation.tick();
    }
    simulation.component::<Position>().y
}

#[test]
fn test_swimming_dives_and_surfaces_based_on_pitch() {
    let _lock = init();

    let looking_down = swim_with_pitch(60.);
    let looking_up = swim_with_pitch(-60.);
    assert!(
        looking_down < looking_up - 0.5,
        "looking down ended at y={looking_down}, looking up ended at y={looking_up}"
    );
}
