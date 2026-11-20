use ::necs::ecs::World;
use necs::{
    ecs::query::{Query, Read, Write},
    register_components,
};

#[derive(Debug, PartialEq, Copy, Clone)]
struct Position {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Debug, PartialEq, Copy, Clone)]
struct Health {
    val: i32,
}

#[derive(Debug, PartialEq, Copy, Clone)]
struct PositionButF32 {
    x: f32,
    y: f32,
    z: f32,
}

register_components!(Health, Position, PositionButF32);

fn handle_movement(_: Query<(), Read>, mut query: Query<(Position,), Write>) {
    println!("handle_movement running...");
    for it in query.iter() {
        println!("Position: {:#?}", it)
    }
}

fn handle_health(_: Query<(), Read>, mut query: Query<(Position, Health), Write>) {
    println!("handle_health running...");
    for (it, sit) in query.iter() {
        println!("Position: {:#?}, Health: {:#?}", it, sit);
    }
}

#[test]
fn full_test() {
    let mut world = World::new()
        .add_system(handle_movement)
        .add_system(handle_health)
        .init();

    let first_entity = world.spawn();
    let second_entity = world.spawn();
    let third_entity = world.spawn();

    let health = Health { val: 100 };
    let position = Position {
        x: 101.1,
        y: 19415.0,
        z: 194.999,
    };
    let position_but_f32 = PositionButF32 {
        x: 999.0,
        y: 999.0,
        z: 999.0,
    };

    // COMPONENT CREATION
    // HEALTH
    world.add_component(first_entity, health);
    world.add_component(second_entity, health);
    world.add_component(third_entity, health);
    // POSITION
    world.add_component(first_entity, position);
    world.add_component(second_entity, position);
    world.add_component(third_entity, position);
    // POSBUTF32
    world.add_component(third_entity, position_but_f32);

    // QUERY
    let mut query = world.query::<(Position, Health)>();
    let mut i = 0;
    for (p, h) in query.iter() {
        assert_eq!(*p, position);
        assert_eq!(*h, health);

        i += 1;
    }
    assert_eq!(i, 3);

    world.run_systems();

    world.despawn(first_entity);
    world.despawn(second_entity);
}

#[test]
fn add_remove_test() {
    let mut world = World::new()
        .add_system(handle_movement)
        .add_system(handle_health)
        .init();

    let first_entity = world.spawn();
    let second_entity = world.spawn();
    let third_entity = world.spawn();

    let health = Health { val: 100 };
    let position = Position {
        x: 101.1,
        y: 19415.0,
        z: 194.999,
    };
    let position_but_f32 = PositionButF32 {
        x: 999.0,
        y: 999.0,
        z: 999.0,
    };

    // COMPONENT CREATION
    // HEALTH
    world.add_component(first_entity, health);
    world.add_component(second_entity, health);
    world.add_component(third_entity, health);
    // POSITION
    world.add_component(first_entity, position);
    world.add_component(second_entity, position);
    world.add_component(third_entity, position);
    // POSBUTF32
    world.add_component(third_entity, position_but_f32);

    // QUERY
    let mut query = world.query::<(Position, Health)>();
    let mut i = 0;
    for (p, h) in query.iter() {
        assert_eq!(*p, position);
        assert_eq!(*h, health);

        i += 1;
    }
    assert_eq!(i, 3);

    // COMPONENT REMOVAL
    world.remove_component::<Position>(first_entity);
    world.remove_component::<Position>(second_entity);
    world.remove_component::<Position>(third_entity);

    world.run_systems();
}
