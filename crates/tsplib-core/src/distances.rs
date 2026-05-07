//! Distance calculation functions for coordinates in 2D and 3D space,
//! as well as for specific distance types defined in the TSPLIB specification.

/// Calculates the Euclidean distance between two nodes in 2D space.
///
/// # Arguments
/// * `(id_1, x_1, y_1)` - A tuple containing the ID and coordinates of the first node.
/// * `(id_2, x_2, y_2)` - A tuple containing the ID and coordinates of the second node.
///
/// # Returns
/// * `i32` - The calculated distance rounded to the nearest integer.
///   Returns 0 if the nodes are the same (i.e., have the same ID).
pub(super) fn distance_euc_2d(
    (id_1, x_1, y_1): (usize, f64, f64),
    (id_2, x_2, y_2): (usize, f64, f64),
) -> i32 {
    if id_1 != id_2 {
        let xd = x_1 - x_2;
        let yd = y_1 - y_2;
        return (xd.powi(2) + yd.powi(2)).sqrt().round() as i32;
    }
    0
}

/// Calculates the Euclidean distance between two nodes in 3D space.
///
/// # Arguments
/// * `(id_1, x_1, y_1, z_1)` - A tuple containing the ID and coordinates of the first node.
/// * `(id_2, x_2, y_2, z_2)` - A tuple containing the ID and coordinates of the second node.
///
/// # Returns
/// * `i32` - The calculated distance rounded to the nearest integer.
///   Returns 0 if the nodes are the same (i.e., have the same ID).
///   If either node does not have a z-coordinate, it is treated as 0 for the distance calculation.
pub(super) fn _distance_euc_3d(
    (id_1, x_1, y_1, z_1): (usize, f64, f64, Option<f64>),
    (id_2, x_2, y_2, z_2): (usize, f64, f64, Option<f64>),
) -> i32 {
    if id_1 != id_2 {
        let xd = x_1 - x_2;
        let yd = y_1 - y_2;
        let zd = z_1.unwrap_or(0.0) - z_2.unwrap_or(0.0);
        return (xd.powi(2) + yd.powi(2) + zd.powi(2)).sqrt().round() as i32;
    }
    0
}

/// Calculates the maximum distance between two nodes in 2D space.
///
/// # Arguments
/// * `(id_1, x_1, y_1)` - A tuple containing the ID and coordinates of the first node.
/// * `(id_2, x_2, y_2)` - A tuple containing the ID and coordinates of the second node.
///
/// # Returns
/// * `i32` - The calculated distance rounded to the nearest integer.
///   Returns 0 if the nodes are the same (i.e., have the same ID).
pub(super) fn distance_max_2d(
    (id_1, x_1, y_1): (usize, f64, f64),
    (id_2, x_2, y_2): (usize, f64, f64),
) -> i32 {
    if id_1 != id_2 {
        let xd = (x_1 - x_2).abs();
        let yd = (y_1 - y_2).abs();
        return (xd.round()).max(yd.round()) as i32;
    }
    0
}

/// Calculates the maximum distance between two nodes in 3D space.
///
/// # Arguments
/// * `(id_1, x_1, y_1, z_1)` - A tuple containing the ID and coordinates of the first node.
/// * `(id_2, x_2, y_2, z_2)` - A tuple containing the ID and coordinates of the second node.
///
/// # Returns
/// * `i32` - The calculated distance rounded to the nearest integer.
///   Returns 0 if the nodes are the same (i.e., have the same ID).
///   If either node does not have a z-coordinate, it is treated as 0 for the distance calculation.
pub(super) fn _distance_max_3d(
    (id_1, x_1, y_1, z_1): (usize, f64, f64, Option<f64>),
    (id_2, x_2, y_2, z_2): (usize, f64, f64, Option<f64>),
) -> i32 {
    if id_1 != id_2 {
        let xd = (x_1 - x_2).abs();
        let yd = (y_1 - y_2).abs();
        let zd = (z_1.unwrap_or(0.0) - z_2.unwrap_or(0.0)).abs();
        return (xd.round()).max(yd.round()).max(zd.round()) as i32;
    }
    0
}

/// Calculates the Manhattan distance between two nodes in 2D space.
///
/// # Arguments
/// * `(id_1, x_1, y_1)` - A tuple containing the ID and coordinates of the first node.
/// * `(id_2, x_2, y_2)` - A tuple containing the ID and coordinates of the second node.
///
/// # Returns
/// * `i32` - The calculated distance rounded to the nearest integer.
///   Returns 0 if the nodes are the same (i.e., have the same ID).
pub(super) fn distance_man_2d(
    (id_1, x_1, y_1): (usize, f64, f64),
    (id_2, x_2, y_2): (usize, f64, f64),
) -> i32 {
    if id_1 != id_2 {
        let xd = (x_1 - x_2).abs();
        let yd = (y_1 - y_2).abs();
        return (xd + yd).round() as i32;
    }
    0
}

/// Calculates the Manhattan distance between two nodes in 3D space.
///
/// # Arguments
/// * `(id_1, x_1, y_1, z_1)` - A tuple containing the ID and coordinates of the first node.
/// * `(id_2, x_2, y_2, z_2)` - A tuple containing the ID and coordinates of the second node.
///
/// # Returns
/// * `i32` - The calculated distance rounded to the nearest integer.
///   Returns 0 if the nodes are the same (i.e., have the same ID).
///   If either node does not have a z-coordinate, it is treated as 0 for the distance calculation.
pub(super) fn _distance_man_3d(
    (id_1, x_1, y_1, z_1): (usize, f64, f64, Option<f64>),
    (id_2, x_2, y_2, z_2): (usize, f64, f64, Option<f64>),
) -> i32 {
    if id_1 != id_2 {
        let xd = (x_1 - x_2).abs();
        let yd = (y_1 - y_2).abs();
        let zd = (z_1.unwrap_or(0.0) - z_2.unwrap_or(0.0)).abs();
        return (xd + yd + zd).round() as i32;
    }
    0
}

/// Calculates the Euclidean distance between two nodes in 2D space and rounds it up to the nearest integer.
///
/// # Arguments
/// * `(id_1, x_1, y_1)` - A tuple containing the ID and coordinates of the first node.
/// * `(id_2, x_2, y_2)` - A tuple containing the ID and coordinates of the second node.
///
/// # Returns
/// * `i32` - The calculated distance rounded up to the nearest integer.
///   Returns 0 if the nodes are the same (i.e., have the same ID).
pub(super) fn distance_ceil_2d(
    (id_1, x_1, y_1): (usize, f64, f64),
    (id_2, x_2, y_2): (usize, f64, f64),
) -> i32 {
    if id_1 != id_2 {
        let xd = x_1 - x_2;
        let yd = y_1 - y_2;
        return (xd.powi(2) + yd.powi(2)).sqrt().ceil() as i32;
    }
    0
}

/// Calculates the pseudo-Euclidean distance between two nodes as
/// defined in the TSPLIB specification for ATT coordinates.
///
/// # Arguments
/// * `(id_1, x_1, y_1)` - A tuple containing the ID and coordinates of the first node.
/// * `(id_2, x_2, y_2)` - A tuple containing the ID and coordinates of the second node.
///
/// # Returns
/// * `i32` - The calculated distance rounded to the nearest integer.
///   Returns 0 if the nodes are the same (i.e., have the same ID).
pub(super) fn distance_att(
    (_, x_1, y_1): (usize, f64, f64),
    (_, x_2, y_2): (usize, f64, f64),
) -> i32 {
    let xd = x_1 - x_2;
    let yd = y_1 - y_2;
    let rij = ((xd.powi(2) + yd.powi(2)) / 10.0).sqrt();
    let tij = rij.round() as i32;
    if (tij as f64) < rij {
        return tij + 1;
    }
    tij
}

/// Calculate the geographical distance between two nodes
/// using the formula provided in the TSPLIB specification for GEO coordinates.
///
/// # Arguments
/// * `(id_1, x_1, y_1)` - A tuple containing the ID and coordinates of the first node.
/// * `(id_2, x_2, y_2)` - A tuple containing the ID and coordinates of the second node.
///
/// # Returns
/// * `i32` - The calculated distance rounded to the nearest integer.
///   Returns 0 if the nodes are the same (i.e., have the same ID).
pub(super) fn distance_geo(
    (id_1, x_1, y_1): (usize, f64, f64),
    (id_2, x_2, y_2): (usize, f64, f64),
) -> i32 {
    if id_1 == id_2 {
        return 0;
    }

    // calculate latitude and longitude in radians for self
    let (lat_1, long_1) = calculate_latitude_longitude(x_1, y_1);

    // calculate latitude and longitude in radians for other
    let (lat_2, long_2) = calculate_latitude_longitude(x_2, y_2);

    // distance calculation
    // RRR = 6378.388;
    // q1 = cos( longitude[i] - longitude[j] );
    // q2 = cos( latitude[i] - latitude[j] );
    // q3 = cos( latitude[i] + latitude[j] );
    // dij = (int) ( RRR * acos( 0.5*((1.0+q1)*q2 - (1.0-q1)*q3)) + 1.0);
    let rrr = 6378.388;
    let q1 = (long_1 - long_2).cos();
    let q2 = (lat_1 - lat_2).cos();
    let q3 = (lat_1 + lat_2).cos();

    // The argument to acos can sometimes be slightly outside the range [-1, 1] due to floating-point precision issues,
    // so we clamp it to ensure it stays within the valid range for acos to avoid NaN results.
    let arg = (0.5 * ((1.0 + q1) * q2 - (1.0 - q1) * q3)).clamp(-1.0, 1.0);
    (rrr * arg.acos() + 1.0) as i32
}

/// Calculates the latitude and longitude in radians for a node based on its x and y
/// coordinates using the formula provided in the TSPLIB documentation for GEO coordinates.
///
/// The formula from the TSPLIB95 specification is as follows:
/// ```
/// PI = 3.141592;
/// deg = nint( x[i] );
/// min = x[i] - deg;
/// latitude[i] = PI * (deg + 5.0 * min / 3.0 ) / 180.0;
/// deg = nint( y[i] );
/// min = y[i] - deg;
/// longitude[i] = PI * (deg + 5.0 * min / 3.0 ) / 180.0;
/// ```
///
/// # Returns
/// * `(f64, f64)` - A tuple containing the latitude and longitude in radians.
#[allow(clippy::approx_constant)]
fn calculate_latitude_longitude(x: f64, y: f64) -> (f64, f64) {
    let pi = 3.141592; // TSPLIB95 specification value for PI

    let deg_x = x.round();
    let min_x = x - deg_x;
    let latitude = pi * (deg_x + 5.0 * min_x / 3.0) / 180.0;

    let deg_y = y.round();
    let min_y = y - deg_y;
    let longitude = pi * (deg_y + 5.0 * min_y / 3.0) / 180.0;

    (latitude, longitude)
}
