extern crate app_units;
#[macro_use]
extern crate bitflags;
extern crate euclid;

pub mod style;
pub mod logical_geometry;

pub use app_units::Au;

pub struct DisplayNodeId(usize);

pub struct DisplayNode {
    pub id: DisplayNodeId,
    pub style: style::ComputedStyle,
}

pub struct DisplayTree {

}
