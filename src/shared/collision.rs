use std::collections::HashMap;
use std::hash::Hash;

#[derive(Copy, Clone, Debug, Default, Hash, Eq, PartialEq)]
pub struct ColliderHandle(u32);

impl ColliderHandle {
    fn next(&mut self) -> Self {
        let next = ColliderHandle(self.0);
        self.0 += 1;
        next
    }
}

pub trait ColliderGroup {
    fn apply(t0: &Self, t1: &Self) -> bool;
}

impl ColliderGroup for u8 {
    fn apply(t0: &Self, t1: &Self) -> bool {
        t0 & t1 != 0
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Collider {
    Circle { x: f32, y: f32, r: f32 },
}

impl Collider {
    fn detect(c0: &Self, c1: &Self) -> bool {
        match (c0, c1) {
            // Circle <=> Circle
            (
                Collider::Circle {
                    x: x0,
                    y: y0,
                    r: r0,
                },
                Collider::Circle {
                    x: x1,
                    y: y1,
                    r: r1,
                },
            ) => {
                let dx = x1 - x0;
                let dy = y1 - y0;
                let sr = r0 + r1;
                dx * dx + dy * dy <= sr * sr
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CollisionGroup<Group, Data> {
    counter: ColliderHandle,
    index_map: HashMap<ColliderHandle, u16>,

    // Data.
    handles: Vec<ColliderHandle>,
    groups: Vec<Group>,
    targets: Vec<Group>,
    colliders: Vec<Collider>,
    data: Vec<Data>,
}

impl<Group: Default + ColliderGroup, Data: Default> CollisionGroup<Group, Data> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        group: Group,
        target: Group,
        data: Data,
        collider: Collider,
    ) -> ColliderHandle {
        let handle = self.counter.next();
        let index = self.handles.len();

        self.index_map.insert(handle, index as _);
        self.handles.push(handle);
        self.groups.push(group);
        self.targets.push(target);
        self.colliders.push(collider);
        self.data.push(data);
        handle
    }

    pub fn unregister(&mut self, handle: ColliderHandle) {
        let Some(index) = self.index_map.remove(&handle) else {
            // Handle not in container.
            return;
        };

        self.handles.swap_remove(index as usize);
        self.groups.swap_remove(index as usize);
        self.targets.swap_remove(index as usize);
        self.colliders.swap_remove(index as usize);
        self.data.swap_remove(index as usize);

        // Correct the index_map.
        self.handles
            .get(index as usize)
            .and_then(|handle| self.index_map.get_mut(handle))
            .map(|ix| *ix = index);
    }

    pub fn generate_contact_events(&self) -> HashMap<ColliderHandle, Vec<&Data>> {
        let mut out: HashMap<ColliderHandle, Vec<&Data>> = HashMap::new();
        let len = self.handles.len();
        for i in 0..len {
            for j in i + 1..len {
                // I -> J
                {
                    let target = &self.targets[i];
                    let group = &self.groups[j];
                    if Group::apply(target, group) {
                        let collider_i = &self.colliders[i];
                        let collider_j = &self.colliders[j];
                        if Collider::detect(collider_i, collider_j) {
                            let handle_i = &self.handles[i];
                            let data_j = &self.data[j];
                            out.entry(*handle_i).or_default().push(&data_j);
                        }
                    }
                }
                // J -> I
                {
                    let target = &self.targets[j];
                    let group = &self.groups[i];
                    if Group::apply(target, group) {
                        let collider_i = &self.colliders[i];
                        let collider_j = &self.colliders[j];
                        if Collider::detect(collider_i, collider_j) {
                            let handle_j = &self.handles[j];
                            let data_i = &self.data[i];
                            out.entry(*handle_j).or_default().push(&data_i);
                        }
                    }
                }
            }
        }

        return out;
    }
}

mod test {
    

    #[test]
    fn collision_test() {
        let mut col_sys = CollisionGroup::new();
        let team0hit = 0b0001_u8;
        let team0hurt = 0b0010_u8;
        let team1hit = 0b0100_u8;
        let team1hurt = 0b1000_u8;
        let hurt0 = col_sys.register(
            team0hurt,
            0,
            0,
            Collider::Circle {
                x: 0.,
                y: 0.,
                r: 2.,
            },
        );
        let hurt1 = col_sys.register(
            team0hurt,
            0,
            0,
            Collider::Circle {
                x: 5.,
                y: 5.,
                r: 2.,
            },
        );
        let hit0 = col_sys.register(
            team0hit,
            team1hurt,
            1,
            Collider::Circle {
                x: 3.,
                y: 0.,
                r: 2.,
            },
        );
        let hit1 = col_sys.register(
            team1hit,
            team0hurt,
            2,
            Collider::Circle {
                x: 3.,
                y: 0.,
                r: 2.,
            },
        );
        let hithurt0 = col_sys.register(
            team1hit | team1hurt,
            team1hurt,
            3,
            Collider::Circle {
                x: 3.,
                y: 0.,
                r: 2.,
            },
        );
        let hithurt1 = col_sys.register(
            team1hit | team1hurt,
            team1hurt,
            4,
            Collider::Circle {
                x: 3.,
                y: 0.,
                r: 2.,
            },
        );

        let events = col_sys.generate_contact_events();

        let events0 = events.get(&hit0);
        let events1 = events.get(&hit1);
        let events2 = events.get(&hurt0);
        let events3 = events.get(&hurt1);
        let events4 = events.get(&hithurt0);
        let events5 = events.get(&hithurt1);

        assert_eq!(events0, Some(&vec![&3, &4]));
        assert_eq!(events1, Some(&vec![&0]));
        assert_eq!(events2, None);
        assert_eq!(events3, None);
        assert_eq!(events4, Some(&vec![&4]));
        assert_eq!(events5, Some(&vec![&3]));
    }
}
