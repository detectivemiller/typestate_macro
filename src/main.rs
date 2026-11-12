// typestate! {
//     #[abbr = "RCT"]
//     rocket = { 
//         [
//             Ground { voltage: u8 }, 
//             Fueling, 
//             Armed, 
//             #[shared] InFlight
//         ], 
//         default = Ground { voltage: 4 } 
//     };

//     #[abbr = "FUL"]
//     fuel = { 
//         [Oil, Wood, #[shared] None], 
//         default = Oil 
//     };

//     struct Rocket {
//         name: String,
//     }

//     impl Rocket {
//         #[transition(space = rocket, from = Ground, to = Fueling)]
//         fn prepare(self) -> Self {
//             // ...
//         }
//     }
// }

use std::collections::HashSet;

use macros::typestate;

typestate! {
        #[abbr = "RCT"]
        rocket = { 
            [
                #[derive(Hash)]
                Ground { pub voltage: u8 }, 
                Fueling, 
                Armed, 
                #[shared] InFlight
            ], 
            default = Ground { voltage: 4 } 
        };

        #[abbr = "FUL"]
        fuel = { 
            [Oil, Wood, #[shared] None], 
            default = Oil 
        };

        struct Rocket {
            pub name: String,
            pub count: u8
        }

        impl Rocket {
            #[transition(space = rocket, from = Ground, to = Fueling)]
            fn prepare(self) -> Self {
                self.transition_rocket(rocket::Fueling)
            }

            #[transition(space = rocket, from = Fueling, to = Armed)]
            fn arm(mut self) -> Self {
                self.name = "fueled".to_string();
                self.transition_rocket(rocket::Armed)
            }
        }
    }

fn main() {
    
}

#[cfg(test)]
    mod tests {
        use crate::{fuel::FuelState, rocket::RocketState};

use super::*;
        #[test]
        fn test_rocket_lifecycle() {
            let rocket_initial = Rocket::new("Saturn V".to_string(), 4);
            
            assert_eq!(rocket_initial.rocket_state(), "Ground");
            assert_eq!(rocket_initial.fuel_state(), "Oil");
            assert_eq!(rocket_initial.name, "Saturn V");
            assert_eq!(rocket_initial.rocket_state.voltage, 4);

            let rocket_fueled = rocket_initial.prepare();
            assert_eq!(rocket_fueled.rocket_state(), "Fueling");

            let rocket_armed = rocket_fueled.arm();
            assert_eq!(rocket_armed.rocket_state(), "Armed");
            assert_eq!(rocket_armed.name, "fueled");
        }
    }
