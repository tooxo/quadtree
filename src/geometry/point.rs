// Copyright 2019 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::geometry::quadrant::Quadrant;

// Transparent alias. In docs and user-facing APIs, this resolves to (U, U).
pub type PointType<U> = (U, U);

// Lightweight data type to represent a point. Should be passed by value.
#[derive(PartialEq, Clone, Copy)]
pub struct Point<U> {
    inner: PointType<U>,
}

impl<U> std::fmt::Debug for Point<U>
where
    U: num::PrimInt + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.inner)
    }
}

impl<U> From<PointType<U>> for Point<U>
where
    U: num::PrimInt,
{
    fn from(xy: PointType<U>) -> Self {
        Point { inner: xy }
    }
}

impl<U> Into<PointType<U>> for Point<U>
where
    U: num::PrimInt,
{
    fn into(self) -> PointType<U> {
        self.inner
    }
}

impl<U> std::ops::Add for Point<U>
where
    U: num::PrimInt,
{
    type Output = Point<U>;
    fn add(self, other: Point<U>) -> Point<U> {
        let new_x: U = self.x() + other.x();
        let new_y: U = self.y() + other.y();
        Point {
            inner: (new_x, new_y),
        }
    }
}

impl<U> std::ops::Sub for Point<U>
where
    U: num::PrimInt,
{
    type Output = Point<U>;
    fn sub(self, other: Point<U>) -> Point<U> {
        let new_x: U = self.x() - other.x();
        let new_y: U = self.y() - other.y();
        Point {
            inner: (new_x, new_y),
        }
    }
}

impl<U> Point<U>
where
    U: num::PrimInt,
{
    // Accessors //
    pub fn x(&self) -> U {
        self.inner.0
    }

    pub fn y(&self) -> U {
        self.inner.1
    }

    // XY::dir_towards() expects that:
    //   - (0, 0) is in the top-left corner,
    //   - +x is right (east),
    //   - -y is down (south).
    //
    // Returns 0 if east or northeast of self,
    //         1 if north or northwest of self,
    //         2 if south or southeast of self,
    //      or 3 if west or southwest of self.
    //
    // Right now this is only used in tests.
    #[allow(dead_code)]
    pub fn dir_towards(&self, other: Point<U>) -> Quadrant {
        if other.x() > self.x() && other.y() <= self.y() {
            Quadrant::Northeast // 0 // east, northeast
        } else if other.x() <= self.x() && other.y() < self.y() {
            Quadrant::Northwest // 1 // north, northwest
        } else if other.x() < self.x() && other.y() >= self.y() {
            Quadrant::Southwest // 3 // west, southwest
        } else {
            Quadrant::Southeast // 2 // south, southeast
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Point;

    #[test]
    fn xy_addition() {
        debug_assert_eq!(Point::from((0, 0)) + Point::from((0, 1)), (0, 1).into());
        debug_assert_eq!(Point::from((0, 1)) + Point::from((0, 1)), (0, 2).into());
        debug_assert_eq!(Point::from((1, 1)) + Point::from((0, 0)), (1, 1).into());
        debug_assert_eq!(Point::from((1, 0)) + Point::from((0, 1)), (1, 1).into());
        debug_assert_eq!(Point::from((0, 0)) + Point::from((4, 5)), (4, 5).into());
        debug_assert_eq!(Point::from((4, 5)) + Point::from((0, 0)), (4, 5).into());
    }

    #[test]
    fn xy_subtraction() {
        debug_assert_eq!(Point::from((0, 1)) - Point::from((0, 0)), (0, 1).into());
        debug_assert_eq!(Point::from((0, 1)) - Point::from((0, 1)), (0, 0).into());
        debug_assert_eq!(Point::from((1, 1)) - Point::from((0, 0)), (1, 1).into());
        debug_assert_eq!(Point::from((1, 1)) - Point::from((0, 1)), (1, 0).into());
        debug_assert_eq!(Point::from((4, 5)) - Point::from((2, 2)), (2, 3).into());
        debug_assert_eq!(Point::from((4, 5)) - Point::from((0, 0)), (4, 5).into());
    }

    // Test addition / subtraction which reaches into the realm of negative numbers.

    #[test]
    fn subtracting_positive_numbers() {
        debug_assert_eq!(Point::from((0, 0)) - (1, 1).into(), (-1, -1).into());
        debug_assert_eq!(Point::from((0, 0)) - (0, 1).into(), (0, -1).into());
        debug_assert_eq!(Point::from((0, 0)) - (1, 0).into(), (-1, 0).into());

        debug_assert_eq!(Point::from((1, 10)) - (2, 20).into(), (-1, -10).into());
    }

    #[test]
    fn adding_negative_numbers() {
        debug_assert_eq!(Point::from((0, 0)) + (-1, 0).into(), (-1, 0).into());
        debug_assert_eq!(Point::from((0, 0)) + (-1, -1).into(), (-1, -1).into());
        debug_assert_eq!(Point::from((0, 0)) + (0, -1).into(), (0, -1).into());

        debug_assert_eq!(Point::from((1, 10)) + (-2, -20).into(), (-1, -10).into());
    }
}

// XY::dir_towards() expects that:
//   - (0, 0) is in the top-left corner,
//   - +x is right (east),
//   - -y is down (south).
//
// This test suite ensures that the following segmentation of the grid around a given point is true
// in all four [plane quadrants](https://en.wikipedia.org/wiki/Quadrant_(plane_geometry)):
//
//          N
//          |
//          |                      |
//       11 1 00                   |
//       11 1 00              III  |  IV
//                                 |
//  W ---33 P 00--> E,x    --------+-------->x
//                                 |
//       33 2 22               II  |  I
//       33 2 22                   |
//          |                      |
//          v                      v
//          S,y                    y
//
#[cfg(test)]
mod quadrant_tests {
    use super::Point;
    use crate::geometry::quadrant::Quadrant;

    #[test]
    fn dir_towards_in_quadrant_i() {
        let origin: Point<i8> = (2, 2).into();

        debug_assert_eq!(origin.dir_towards((2, 1).into()), Quadrant::Northwest); // Due north
        debug_assert_eq!(origin.dir_towards((3, 1).into()), Quadrant::Northeast); // Northeast
        debug_assert_eq!(origin.dir_towards((3, 2).into()), Quadrant::Northeast); // Due east
        debug_assert_eq!(origin.dir_towards((3, 3).into()), Quadrant::Southeast); // Southeast
        debug_assert_eq!(origin.dir_towards((2, 3).into()), Quadrant::Southeast); // Due south
        debug_assert_eq!(origin.dir_towards((1, 3).into()), Quadrant::Southwest); // Southwest
        debug_assert_eq!(origin.dir_towards((1, 2).into()), Quadrant::Southwest); // Due west
        debug_assert_eq!(origin.dir_towards((1, 1).into()), Quadrant::Northwest); // Northwest
    }

    #[test]
    fn dir_towards_in_quadrant_ii() {
        let origin: Point<i8> = (-2, 2).into();

        debug_assert_eq!(origin.dir_towards((-2, 1).into()), Quadrant::Northwest); // Due north
        debug_assert_eq!(origin.dir_towards((-1, 1).into()), Quadrant::Northeast); // Northeast
        debug_assert_eq!(origin.dir_towards((-1, 2).into()), Quadrant::Northeast); // Due east
        debug_assert_eq!(origin.dir_towards((-1, 3).into()), Quadrant::Southeast); // Southeast
        debug_assert_eq!(origin.dir_towards((-2, 3).into()), Quadrant::Southeast); // Due south
        debug_assert_eq!(origin.dir_towards((-3, 3).into()), Quadrant::Southwest); // Southwest
        debug_assert_eq!(origin.dir_towards((-3, 2).into()), Quadrant::Southwest); // Due west
        debug_assert_eq!(origin.dir_towards((-3, 1).into()), Quadrant::Northwest); // Northwest
    }

    #[test]
    fn dir_towards_in_quadrant_iii() {
        let origin: Point<i8> = (-2, -2).into();

        debug_assert_eq!(origin.dir_towards((-2, -3).into()), Quadrant::Northwest); // Due north
        debug_assert_eq!(origin.dir_towards((-1, -3).into()), Quadrant::Northeast); // Northeast
        debug_assert_eq!(origin.dir_towards((-1, -2).into()), Quadrant::Northeast); // Due east
        debug_assert_eq!(origin.dir_towards((-1, -1).into()), Quadrant::Southeast); // Southeast
        debug_assert_eq!(origin.dir_towards((-2, -1).into()), Quadrant::Southeast); // Due south
        debug_assert_eq!(origin.dir_towards((-3, -1).into()), Quadrant::Southwest); // Southwest
        debug_assert_eq!(origin.dir_towards((-3, -2).into()), Quadrant::Southwest); // Due west
        debug_assert_eq!(origin.dir_towards((-3, -3).into()), Quadrant::Northwest); // Northwest
    }

    #[test]
    fn dir_towards_in_quadrant_iv() {
        let origin: Point<i8> = (2, -2).into();

        debug_assert_eq!(origin.dir_towards((2, -3).into()), Quadrant::Northwest); // Due north
        debug_assert_eq!(origin.dir_towards((3, -3).into()), Quadrant::Northeast); // Northeast
        debug_assert_eq!(origin.dir_towards((3, -2).into()), Quadrant::Northeast); // Due east
        debug_assert_eq!(origin.dir_towards((3, -1).into()), Quadrant::Southeast); // Southeast
        debug_assert_eq!(origin.dir_towards((2, -1).into()), Quadrant::Southeast); // Due south
        debug_assert_eq!(origin.dir_towards((1, -1).into()), Quadrant::Southwest); // Southwest
        debug_assert_eq!(origin.dir_towards((1, -2).into()), Quadrant::Southwest); // Due west
        debug_assert_eq!(origin.dir_towards((1, -3).into()), Quadrant::Northwest); // Northwest
    }

    #[test]
    fn dir_towards_from_origin() {
        let origin: Point<i8> = (0, 0).into();

        debug_assert_eq!(origin.dir_towards((0, -1).into()), Quadrant::Northwest); // Due north
        debug_assert_eq!(origin.dir_towards((1, -1).into()), Quadrant::Northeast); // Northeast
        debug_assert_eq!(origin.dir_towards((1, 0).into()), Quadrant::Northeast); // Due east
        debug_assert_eq!(origin.dir_towards((1, 1).into()), Quadrant::Southeast); // Southeast
        debug_assert_eq!(origin.dir_towards((0, 1).into()), Quadrant::Southeast); // Due south
        debug_assert_eq!(origin.dir_towards((-1, 1).into()), Quadrant::Southwest); // Southwest
        debug_assert_eq!(origin.dir_towards((-1, 0).into()), Quadrant::Southwest); // Due west
        debug_assert_eq!(origin.dir_towards((-1, -1).into()), Quadrant::Northwest); // Northwest
    }
}
