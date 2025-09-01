pub mod sgp4;
pub mod eop;
pub mod coordinates;

pub use sgp4::{OrbitalElements, TleParser, OmmParser, Sgp4Propagator, StateVector, Sgp4Error};
pub use eop::{EopData, EopCache};
pub use coordinates::{CoordinateContext, CoordinateError, teme_to_gcrs, gcrs_to_itrs, teme_to_itrs};

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
