//! Reading Guitar Pro files into a [`model::Song`]: GP3, GP4 and GP5 in
//! `gp345`, GP6 (`.gpx`) and GP7 (`.gp`) in `gp67`.

pub mod gp345;
pub mod gp67;
pub mod model;
mod parse;

// Top-level parsing entry point (dispatches by container format).
pub use parse::parse_gp_data;

#[cfg(test)]
pub mod test_support;
#[cfg(test)]
mod tests;
