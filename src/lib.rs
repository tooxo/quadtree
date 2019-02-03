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

//! [Point/region Quadtree](https://en.wikipedia.org/wiki/Quadtree) implementation
//! for Rust.
//!
//! # Quick Start
//!
//! Add `quadtree_impl` to your `Cargo.toml`, and then add it to your main.
//! ```
//! extern crate quadtree_impl;
//!
//! use quadtree_impl::Quadtree;
//!
//! // Create a new Quadtree with (u16, u16) x/y coordinates, String values, and a depth of four
//! // layers. Since 2^4 = 16, this grid will be of width and height 16.
//! let mut qt = Quadtree::<u64, String>::new(4);
//!
//! // Insert "foo" in the coordinate system such that it occupies a rectangle with top-left
//! // "anchor" (0, 0), and width/height 2x1.
//! //
//! //   0  1  2  3
//! // 0 ░░░░░░░--+
//! //   ░░░░░░░ <--foo
//! // 1 ░░░░░░░--+
//! //   |  |  |  |
//! // 2 +--+--+--+
//! qt.insert((0, 0), (2, 1), "foo".to_string());
//!
//! // Perform a query over a region with anchor (1, 0) and width/height 1x1...
//! //
//! //   0  1  2  3
//! // 0 ░░░▓▓▓▓▒▒▒
//! //   ░░░▓▓▓▓▒▒▒ <--query region
//! // 1 ░░░▓▓▓▓▒▒▒
//! //   |  ▒▒▒▒▒▒▒
//! // 2 +--▒▒▒▒▒▒▒
//! let mut query = qt.query((1, 0), (2, 2));
//!
//! // There is an overlap between our query region and the region holding "foo",
//! // so we expect that iterator to return the `(coordinate, value)` pair containing "foo".
//! assert_eq!(query.next()
//!                 .map_or("", |(_coordinate, value)| value),
//!            "foo");
//! ```
//!
//! # Usage
//!
//! For usage details, see [`Quadtree`].
//!
//! [`Quadtree`]: struct.Quadtree.html

// For extra-pedantic documentation tests.
#![doc(test(attr(deny(warnings))))]

extern crate num;

mod geometry;
mod qtinner;
mod types;
mod uuid_iter;

use crate::geometry::area::{Area, AreaType};
use crate::geometry::point::PointType;
use crate::qtinner::QTInner;
use crate::types::StoreType;
use crate::uuid_iter::UuidIter;
use num::{cast::FromPrimitive, PrimInt};
use std::collections::HashMap;
use std::iter::FusedIterator;
use uuid::Uuid;

//   .d88b.  db    db  .d8b.  d8888b. d888888b d8888b. d88888b d88888b
//  .8P  Y8. 88    88 d8' `8b 88  `8D `~~88~~' 88  `8D 88'     88'
//  88    88 88    88 88ooo88 88   88    88    88oobY' 88ooooo 88ooooo
//  88    88 88    88 88~~~88 88   88    88    88`8b   88~~~~~ 88~~~~~
//  `8P  d8' 88b  d88 88   88 88  .8D    88    88 `88. 88.     88.
//   `Y88'Y8 ~Y8888P' YP   YP Y8888D'    YP    88   YD Y88888P Y88888P

/// A data structure for storing and accessing data by x/y coordinates.
/// (A [Quadtree](https://en.wikipedia.org/wiki/Quadtree).)
///
/// `Quadtree<U, V>` is parameterized over
///  - `U`, where `U` is the index type of the x/y coordinate, and
///  - `V`, where `V` is the value being stored in the data structure.
///
/// Both points and regions are represented by the type
/// ```
/// type U = u64; // Or any primitive integer, signed or unsigned.
///
/// let _region: (/*    anchor=*/ (U, U),
///               /*dimensions=*/ (U, U)) = ((1, 2), (3, 4)); // (for example)
/// ```
/// where
///  - `anchor` is the x/y coordinate of the top-left corner, and
///  - `dimensions` is a tuple containing the width and height of the region.
///
/// Points have dimensions `(1, 1)`.
///
///   - TODO(ambuc): In lieu of mutable getters, expose the held UUID and allow specific lookups
///   - TODO(ambuc): Size hints in iterators
///   - TODO(ambuc): Implement strictly inclusive getters.
///   - TODO(ambuc): Implement `.clear(anchor, size)`.
///   - TODO(ambuc): Implement `.delete(anchor, size)`.
///   - TODO(ambuc): Implement `.delete_by(anchor, size, fn)`.
///   - TODO(ambuc): Implement `.retain(anchor, size, fn)`.
///   - TODO(ambuc): Implement `FromIterator<(K, V)>` for `Quadtree`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quadtree<U, V>
where
    U: PrimInt,
{
    depth: usize,
    inner: QTInner<U>,
    store: StoreType<U, V>,
}

impl<U, V> Quadtree<U, V>
where
    U: PrimInt,
{
    /// Creates a new, empty Quadtree with the requested depth.
    /// - The default anchor is `(0, 0)`, and the default width and height are both `2^depth`.
    /// - The Quadtree must be explicitly typed, since will contain items of a type.
    /// ```
    /// use quadtree_impl::Quadtree;
    ///
    /// let qt = Quadtree::<u32, u8>::new(/*depth=*/ 2);
    ///
    /// assert_eq!(qt.depth(), 2);
    /// assert_eq!(qt.anchor(), (0, 0));
    /// assert_eq!(qt.width(), 4);
    /// assert_eq!(qt.height(), 4);
    /// ```
    pub fn new(depth: usize) -> Quadtree<U, V> {
        Quadtree::new_with_anchor((U::zero(), U::zero()), depth)
    }

    /// Creates a new Quadtree with the requested anchor and depth.
    /// ```
    /// use quadtree_impl::Quadtree;
    ///
    /// let qt = Quadtree::<u32, u8>::new_with_anchor(/*anchor=*/ (2, 4), /*depth=*/ 3);
    ///
    /// assert_eq!(qt.depth(), 3);
    /// assert_eq!(qt.anchor(), (2, 4));
    /// assert_eq!(qt.width(), 8);
    /// assert_eq!(qt.height(), 8);
    /// ```
    pub fn new_with_anchor(anchor: PointType<U>, depth: usize) -> Quadtree<U, V> {
        Quadtree {
            depth,
            inner: QTInner::new(anchor, depth),
            store: HashMap::new(),
        }
    }

    /// The coordinate of the top-left corner of the represented region.
    pub fn anchor(&self) -> PointType<U> {
        self.inner.region.anchor().into()
    }

    /// The width of the represented region.
    pub fn width(&self) -> usize {
        self.inner.region.width().to_usize().unwrap()
    }

    /// The height of the represented region.
    pub fn height(&self) -> usize {
        self.inner.region.height().to_usize().unwrap()
    }

    /// The depth of the quadtree.
    /// - A quadtree created with depth 0 will have one node and no possibility for subdivision;
    /// - a quadtree created with depth 1 will have one node and four
    /// potential subquadrants.
    ///
    /// Thus both the width and height of a quadtree with depth `n` are `2^n`.
    pub fn depth(&self) -> usize {
        self.inner.depth
    }

    /// Returns the number of elements in the quadtree.
    /// ```
    /// use quadtree_impl::Quadtree;
    ///
    /// let mut qt = Quadtree::<u32, f32>::new(4);
    /// assert_eq!(qt.len(), 0);
    ///
    /// qt.insert_pt((3, 1), 3.14159);
    /// assert_eq!(qt.len(), 1);
    ///
    /// qt.insert_pt((2, 7), 2.71828);
    /// assert_eq!(qt.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Whether or not the quadtree is empty.
    /// ```
    /// use quadtree_impl::Quadtree;
    ///
    /// let mut qt = Quadtree::<u32, f64>::new(3);
    /// assert!(qt.is_empty());
    ///
    /// qt.insert((1, 4), (1, 4), 1.4142135);
    /// assert!(!qt.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// Whether or not the region represented by this quadtree could contain the given region.
    ///
    /// The region described may have an anchor anywhere on the plane, but it
    /// must have positive, nonzero values for its width and height.
    ///
    /// Perhaps before inserting a region, the callsite would like to check to see if that region
    /// could fit in the area represented by the quadtree.
    /// ```
    /// //  0  1  2  3  4
    /// // 0+--*******--+
    /// //  |  *******  |
    /// // 1+--*******--+
    /// //  |  *******  |
    /// // 2+--*******--+
    ///
    /// use quadtree_impl::Quadtree;
    ///
    /// let qt = Quadtree::<u32, u32>::new_with_anchor((1, 0), 1);
    ///
    /// assert!(qt.contains((1, 0), (2, 2))); // qt contains itself.
    ///
    /// // qt contains a 1x1 region within it.
    /// //
    /// //  0  1  2  3  4
    /// // 0+--XXXX***--+
    /// //  |  XXXX***  |
    /// // 1+--XXXX***--+
    /// //  |  *******  |
    /// // 2+--*******--+
    /// assert!(qt.contains((1, 0), (1, 1)));
    ///
    /// // But, qt does not contain regions which are not totally within it.
    /// //
    /// //  0  1  2  3  4
    /// // 0XXXX******--+
    /// //  XXXX******  |
    /// // 1XXXX******--+
    /// //  |  *******  |
    /// // 2+--*******--+
    /// assert!(!qt.contains((0, 0), (1, 1)));
    /// ```
    pub fn contains(&self, anchor: PointType<U>, size: (U, U)) -> bool {
        self.inner.region.contains((anchor, size).into())
    }

    /// Attempts to insert the value at the requested anchor and size. Returns false if the region
    /// was too large.
    ///
    /// The region described may have an anchor anywhere on the plane, but it
    /// must have positive, nonzero values for its width and height.
    ///
    /// ```
    /// use quadtree_impl::Quadtree;
    ///
    /// let mut qt = Quadtree::<u32, i64>::new(2);
    ///
    /// // Returns whether or not the requested region fits in the quadtree.
    /// qt.insert((0, 0), (1, 1), 500000);
    /// qt.insert((0, 0), (5, 4), 27500);
    /// ```
    pub fn insert(&mut self, anchor: PointType<U>, size: (U, U), val: V) {
        self.inner
            .insert_val_at_region((anchor, size).into(), val, &mut self.store)
    }

    /// Attempts to insert the value at the given point. Returns false if the point was out of
    /// bounds.
    ///  - Expect the behavior of `.insert_pt(_, _)` to be the same as [`.insert(_, (1, 1), _)`].
    /// ```
    /// use quadtree_impl::Quadtree;
    ///
    /// let mut qt = Quadtree::<u32, i64>::new(2);
    ///
    /// qt.insert_pt((0, 0),  8675309);
    /// qt.insert_pt((5, 4),  6060842);
    /// ```
    ///
    /// [`.insert(_, (1, 1), _)`]: struct.Quadtree.html#method.insert
    pub fn insert_pt(&mut self, anchor: PointType<U>, val: V) {
        self.inner.insert_val_at_region(
            (anchor, Self::default_region_size()).into(),
            val,
            &mut self.store,
        )
    }

    /// Returns an iterator over `(&'a ((U, U), (U, U)), &'a V)` tuples representing values
    /// within the query region.
    ///  - Values returned may either partially intersect or be wholly within the query region.
    ///
    /// The query region described may have an anchor anywhere on the plane, but it
    /// must have positive, nonzero values for its width and height.
    ///
    /// ```
    /// use quadtree_impl::Quadtree;
    ///
    /// let mut qt = Quadtree::<u32, i16>::new(4);
    ///
    /// qt.insert((0, 5), (7, 7), 21);
    /// qt.insert((1, 3), (1, 3), 57);
    ///
    /// // Query over the region anchored at (0, 5) with area 1x1.
    /// let mut query_a = qt.query((0, 5), (1, 1));
    /// assert_eq!(query_a.next(),
    ///            Some((&((0, 5), (7, 7)), &21)));
    /// assert_eq!(query_a.next(),
    ///            None);
    ///
    /// // Query over the region anchored at (0, 0) with area 6x6.
    /// let query_b = qt.query((0, 0), (6, 6));
    ///
    /// // It's unclear what order the regions should return in, but there will be two of them.
    /// assert_eq!(query_b.count(), 2);
    /// ```
    pub fn query(&self, anchor: PointType<U>, size: (U, U)) -> Query<U, V> {
        let a = (anchor, size).into();
        Query::new(a, &self.inner, &self.store)
    }

    /// Returns an iterator (of type [`Query`]) over `(&'a ((U, U), (U, U)), &'a V)` tuples
    /// representing values intersecting the query point.
    ///
    /// Alias for [`.query(anchor, (1, 1))`].
    ///
    /// [`Query`]: struct.Query.html
    /// [`.query(anchor, (1, 1))`]: struct.Quadtree.html#method.query
    pub fn query_pt(&self, anchor: PointType<U>) -> Query<U, V> {
        let a = (anchor, Self::default_region_size()).into();
        Query::new(a, &self.inner, &self.store)
    }

    /// Accepts a modification lambda of type `Fn(&mut V) + Copy` and applies it to all elements in
    /// the Quadtree.
    /// ```
    /// use quadtree_impl::Quadtree;
    ///
    /// let mut qt = Quadtree::<u8, f64>::new(3);
    ///
    /// qt.insert((0, 0), (1, 1), 1.23);
    /// qt.modify_all(|i| *i += 2.0);
    ///
    /// assert_eq!(qt.iter().next(),
    ///            Some((&((0, 0), (1, 1)), &3.23)));
    /// ```
    pub fn modify_all<F>(&mut self, f: F)
    where
        F: Fn(&mut V) + Copy,
    {
        self.modify_region(|_| true, f);
    }

    /// Accepts a modification lambda of type `Fn(&mut V) + Copy` and applies it to all elements in
    /// the Quadtree intersecting the described region.
    pub fn modify<F>(&mut self, anchor: PointType<U>, size: (U, U), f: F)
    where
        F: Fn(&mut V) + Copy,
    {
        let query_region = (anchor, size).into();
        self.modify_region(|a| a.intersects(query_region), f);
    }

    /// Accepts a modification lambda of type `Fn(&mut V) + Copy` and applies it to all elements in
    /// the Quadtree overlapping the given point.
    pub fn modify_pt<F>(&mut self, anchor: PointType<U>, f: F)
    where
        F: Fn(&mut V) + Copy,
    {
        let query_region = (anchor, Self::default_region_size()).into();
        self.modify_region(|a| a.intersects(query_region), f);
    }

    fn modify_region<F, M>(&mut self, filter: F, modify: M)
    where
        F: Fn(Area<U>) -> bool,
        M: Fn(&mut V) + Copy,
    {
        let relevant_uuids: Vec<Uuid> = UuidIter::new(&self.inner).collect();
        for uuid in relevant_uuids {
            if let Some((region, value)) = self.store.get_mut(&uuid) {
                if filter(*region) {
                    modify(value);
                }
            }
        }
    }

    /// Resets the quadtree to a totally empty state.
    pub fn reset(&mut self) {
        self.store.clear();
        self.inner.reset();
    }

    /// Returns an iterator over all `(&((U, U), (U, U)), &V)` region/value pairs in the
    /// Quadtree.
    pub fn iter(&self) -> Iter<U, V>
    where
        U: FromPrimitive,
    {
        Iter::new(&self.inner, &self.store)
    }

    /// Returns an iterator over all `&'a ((U, U), (U, U))` regions in the Quadtree.
    pub fn regions(&self) -> Regions<U, V>
    where
        U: FromPrimitive,
    {
        Regions {
            inner: Iter::new(&self.inner, &self.store),
        }
    }

    /// Returns an iterator over all `&'a V` values in the Quadtree.
    pub fn values(&self) -> Values<U, V>
    where
        U: FromPrimitive,
    {
        Values {
            inner: Iter::new(&self.inner, &self.store),
        }
    }

    // Strongly-typed alias for (one(), one()).
    fn default_region_size() -> (U, U) {
        (U::one(), U::one())
    }
}

// d888888b d888888b d88888b d8888b.
//   `88'   `~~88~~' 88'     88  `8D
//    88       88    88ooooo 88oobY'
//    88       88    88~~~~~ 88`8b
//   .88.      88    88.     88 `88.
// Y888888P    YP    Y88888P 88   YD

/// An iterator over all regions and values of a [`Quadtree`].
///
/// This struct is created by the [`iter`] method on [`Quadtree`].
///
/// [`iter`]: struct.Quadtree.html#method.iter
/// [`Quadtree`]: struct.Quadtree.html
#[derive(Clone, Debug)]
pub struct Iter<'a, U, V>
where
    U: PrimInt,
{
    store: &'a StoreType<U, V>,
    uuid_iter: UuidIter<'a, U>,
}

impl<'a, U, V> Iter<'a, U, V>
where
    U: PrimInt,
{
    pub(crate) fn new(qt: &'a QTInner<U>, store: &'a StoreType<U, V>) -> Iter<'a, U, V> {
        Iter {
            store,
            uuid_iter: UuidIter::new(qt),
        }
    }
}

impl<'a, U, V> Iterator for Iter<'a, U, V>
where
    U: PrimInt,
{
    type Item = (&'a AreaType<U>, &'a V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.uuid_iter.next() {
            Some(uuid) => {
                let (region, value) = self
                    .store
                    .get(&uuid)
                    .expect("Shouldn't have a uuid in the tree which isn't in the store.");
                Some((&region.inner(), &value))
            }
            None => None,
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.uuid_iter.size_hint()
    }
}

impl<U, V> FusedIterator for Iter<'_, U, V> where U: PrimInt {}

impl<U, V> ExactSizeIterator for Iter<'_, U, V>
where
    U: PrimInt,
{
    fn len(&self) -> usize {
        self.uuid_iter.len()
    }
}

//  .d88b.  db    db d88888b d8888b. db    db
// .8P  Y8. 88    88 88'     88  `8D `8b  d8'
// 88    88 88    88 88ooooo 88oobY'  `8bd8'
// 88    88 88    88 88~~~~~ 88`8b      88
// `8P  d8' 88b  d88 88.     88 `88.    88
//  `Y88'Y8 ~Y8888P' Y88888P 88   YD    YP

/// An iterator over the regions and values of a [`Quadtree`].
///
/// This struct is created by the [`query`] or [`query_pt`] methods on [`Quadtree`].
///
/// [`query`]: struct.Quadtree.html#method.query
/// [`query_pt`]: struct.Quadtree.html#method.query_pt
/// [`Quadtree`]: struct.Quadtree.html
#[derive(Clone, Debug)]
pub struct Query<'a, U, V>
where
    U: PrimInt,
{
    pub(crate) query_region: Area<U>,
    pub(crate) uuid_iter: UuidIter<'a, U>,
    store: &'a StoreType<U, V>,
    resource_type: std::marker::PhantomData<V>,
}

impl<'a, U, V> Query<'a, U, V>
where
    U: PrimInt,
{
    pub(crate) fn new(
        query_region: Area<U>,
        qt: &'a QTInner<U>,
        store: &'a StoreType<U, V>,
    ) -> Query<'a, U, V>
    where
        U: PrimInt,
    {
        let q = Query {
            query_region,
            uuid_iter: UuidIter::new(qt),
            store,
            resource_type: std::marker::PhantomData,
        };
        // TODO(ambuc): descend + collect
        q
    }
}

impl<'a, U, V> Iterator for Query<'a, U, V>
where
    U: PrimInt,
{
    type Item = (&'a AreaType<U>, &'a V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // TODO(ambuc): There's an optimization over the inner iterator here -- we don't need to
        // descend the whole tree, just those subquadrants which intersect our held @query_region.
        // This is essential to performance.
        if let Some(uuid) = self.uuid_iter.next() {
            if let Some((region, value)) = self.store.get(&uuid) {
                if self.query_region.intersects(*region) {
                    return Some((region.inner(), value));
                }
                return self.next();
            }
        }
        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.uuid_iter.size_hint()
    }
}

impl<U, V> FusedIterator for Query<'_, U, V> where U: PrimInt {}

// d8888b. d88888b  d888b  d888888b  .d88b.  d8b   db .d8888.
// 88  `8D 88'     88' Y8b   `88'   .8P  Y8. 888o  88 88'  YP
// 88oobY' 88ooooo 88         88    88    88 88V8o 88 `8bo.
// 88`8b   88~~~~~ 88  ooo    88    88    88 88 V8o88   `Y8b.
// 88 `88. 88.     88. ~8~   .88.   `8b  d8' 88  V888 db   8D
// 88   YD Y88888P  Y888P  Y888888P  `Y88P'  VP   V8P `8888Y'

/// An iterator over the regions held within a [`Quadtree`].
///
/// This struct is created by the [`regions`] method on [`Quadtree`].
///
/// [`regions`]: struct.Quadtree.html#method.regions
/// [`Quadtree`]: struct.Quadtree.html
#[derive(Clone, Debug)]
pub struct Regions<'a, U, V>
where
    U: PrimInt,
{
    pub(crate) inner: Iter<'a, U, V>,
}

impl<'a, U, V> Iterator for Regions<'a, U, V>
where
    U: PrimInt,
{
    type Item = (&'a AreaType<U>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map_or(None, |(k, _v)| Some(k))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<U, V> FusedIterator for Regions<'_, U, V> where U: PrimInt {}

impl<U, V> ExactSizeIterator for Regions<'_, U, V>
where
    U: PrimInt,
{
    fn len(&self) -> usize {
        self.inner.len()
    }
}

// db    db  .d8b.  db      db    db d88888b .d8888.
// 88    88 d8' `8b 88      88    88 88'     88'  YP
// Y8    8P 88ooo88 88      88    88 88ooooo `8bo.
// `8b  d8' 88~~~88 88      88    88 88~~~~~   `Y8b.
//  `8bd8'  88   88 88booo. 88b  d88 88.     db   8D
//    YP    YP   YP Y88888P ~Y8888P' Y88888P `8888Y'

/// An iterator over the values held within a [`Quadtree`].
///
/// This struct is created by the [`values`] method on [`Quadtree`].
///
/// [`values`]: struct.Quadtree.html#method.values
/// [`Quadtree`]: struct.Quadtree.html
#[derive(Clone, Debug)]
pub struct Values<'a, U, V>
where
    U: PrimInt,
{
    pub(crate) inner: Iter<'a, U, V>,
}

impl<'a, U, V> Iterator for Values<'a, U, V>
where
    U: PrimInt,
{
    type Item = (&'a V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map_or(None, |(_k, v)| Some(v))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<U, V> FusedIterator for Values<'_, U, V> where U: PrimInt {}

impl<U, V> ExactSizeIterator for Values<'_, U, V>
where
    U: PrimInt,
{
    fn len(&self) -> usize {
        self.inner.len()
    }
}

// d888888b d8b   db d888888b  .d88b.  d888888b d888888b d88888b d8888b.
//   `88'   888o  88 `~~88~~' .8P  Y8.   `88'   `~~88~~' 88'     88  `8D
//    88    88V8o 88    88    88    88    88       88    88ooooo 88oobY'
//    88    88 V8o88    88    88    88    88       88    88~~~~~ 88`8b
//   .88.   88  V888    88    `8b  d8'   .88.      88    88.     88 `88.
// Y888888P VP   V8P    YP     `Y88P'  Y888888P    YP    Y88888P 88   YD

/// A consuming iterator over all region/value pairs held in a [`Quadtree`].
///
/// This struct is created by the `into_iter()` method on the [`IntoIterator`] trait.
///
/// [`IntoIterator`]: struct.Quadtree.html#impl-IntoIterator
///
/// [`Quadtree`]: struct.Quadtree.html
#[derive(Clone, Debug)]
pub struct IntoIter<U, V>
where
    U: PrimInt,
{
    uuid_stack: Vec<Uuid>,
    qt_stack: Vec<QTInner<U>>,
    remaining: usize,
    store: StoreType<U, V>,
}

impl<U, V> IntoIter<U, V>
where
    U: PrimInt,
{
    pub(crate) fn new(qt: QTInner<U>, store: StoreType<U, V>) -> IntoIter<U, V> {
        let len = store.len();
        IntoIter {
            uuid_stack: Vec::new(),
            qt_stack: vec![qt],
            remaining: len,
            store,
        }
    }
}

impl<U, V> Iterator for IntoIter<U, V>
where
    U: PrimInt,
{
    type Item = (AreaType<U>, V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Check the uuid_stack.
        if let Some(uuid) = self.uuid_stack.pop() {
            self.remaining -= 1;

            let (_uuid, (region, value)) = self
                .store
                .remove_entry(&uuid)
                .expect("Shouldn't have a uuid in the tree which isn't in the store.");

            return Some((*region.inner(), value));
        }

        // Then check the qt_stack.
        if let Some(qt) = self.qt_stack.pop() {
            // Push my regions onto the region stack
            self.uuid_stack.extend(qt.kept_uuids);

            // Push my subquadrants onto the qt_stack too.
            if let Some([a, b, c, d]) = qt.subquadrants {
                self.qt_stack.push(*a);
                self.qt_stack.push(*b);
                self.qt_stack.push(*c);
                self.qt_stack.push(*d);
            }

            return self.next();
        }

        // Else there's nothing left to search.
        return None;
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<U, V> ExactSizeIterator for IntoIter<U, V>
where
    U: PrimInt,
{
    #[inline]
    fn len(&self) -> usize {
        self.remaining
    }
}

impl<U, V> FusedIterator for IntoIter<U, V> where U: PrimInt {}

/// `Extend<((U, U), V)>` will silently drop values whose coordinates do not fit in the region
/// represented by the Quadtree. It is the responsibility of the callsite to ensure these points
/// fit.
impl<U, V> Extend<(PointType<U>, V)> for Quadtree<U, V>
where
    U: PrimInt,
{
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = (PointType<U>, V)>,
    {
        for (pt, val) in iter {
            self.insert_pt(pt, val);
        }
    }
}

/// `Extend<(((U, U), (U, U), V)>` will silently drop values whose coordinates do not fit in the
/// region represented by the Quadtree. It is the responsibility of the callsite to ensure these
/// points fit.
impl<U, V> Extend<(AreaType<U>, V)> for Quadtree<U, V>
where
    U: PrimInt,
{
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = (AreaType<U>, V)>,
    {
        for ((anchor, size), val) in iter {
            self.insert(anchor, size, val);
        }
    }
}

// Immutable iterator for the Quadtree, returning by-reference.
impl<'a, U, V> IntoIterator for &'a Quadtree<U, V>
where
    U: PrimInt,
{
    type Item = (&'a AreaType<U>, &'a V);
    type IntoIter = Iter<'a, U, V>;

    fn into_iter(self) -> Iter<'a, U, V> {
        Iter::new(&self.inner, &self.store)
    }
}

impl<U, V> IntoIterator for Quadtree<U, V>
where
    U: PrimInt,
{
    type Item = (AreaType<U>, V);
    type IntoIter = IntoIter<U, V>;

    fn into_iter(self) -> IntoIter<U, V> {
        IntoIter::new(self.inner, self.store)
    }
}
