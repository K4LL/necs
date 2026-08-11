use std::marker::PhantomData;

use ::necs::ecs::{Entity, World};
use necs::{ecs::query::ComponentSet, register_components};

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

#[test]
fn full_test() {
    let mut world = World::new();

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

    world.despawn(first_entity);
    world.despawn(second_entity);
}
