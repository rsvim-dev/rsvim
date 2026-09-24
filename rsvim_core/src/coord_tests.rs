// use super::coord::*;
use crate::prelude::*;
use std::mem;

#[test]
fn cast_geo_points() {
  let p1: IPos = point!(1, 2);
  let actual1 = point_as!(p1, usize);
  let actual1_x = actual1.x();
  let actual1_y = actual1.y();
  assert_eq!(mem::size_of_val(&actual1_x), mem::size_of_val(&1_usize));
  assert_eq!(mem::size_of_val(&actual1_y), mem::size_of_val(&2_usize));

  let p2: U16Pos = point!(15_u16, 25_u16);
  let actual2 = point_as!(p2, i32);
  let actual2_x = actual2.x();
  let actual2_y = actual2.y();
  assert_eq!(mem::size_of_val(&actual2_x), mem::size_of_val(&15_i32));
  assert_eq!(mem::size_of_val(&actual2_y), mem::size_of_val(&25_i32));

  let p3: Point<u32> = point!(78_u32, 88_u32);
  let actual3 = point_as!(p3, i16);
  let actual3_x = actual3.x();
  let actual3_y = actual3.y();
  assert_eq!(mem::size_of_val(&actual3_x), mem::size_of_val(&78_i16));
  assert_eq!(mem::size_of_val(&actual3_y), mem::size_of_val(&88_i16));
}

#[test]
fn cast_geo_rects() {
  let r1: IRect = rect!(1, 2, 3, 4);
  let actual1 = rect_as!(r1, u8);
  let actual1_min = actual1.min();
  let actual1_max = actual1.max();
  assert_eq!(mem::size_of_val(&actual1_min.x), mem::size_of_val(&1_u8));
  assert_eq!(mem::size_of_val(&actual1_min.y), mem::size_of_val(&2_u8));
  assert_eq!(mem::size_of_val(&actual1_max.x), mem::size_of_val(&3_u8));
  assert_eq!(mem::size_of_val(&actual1_max.y), mem::size_of_val(&4_u8));

  let r2: Rect<u16> = rect!(15_u16, 25_u16, 35_u16, 45_u16);
  let actual2 = rect_as!(r2, i32);
  let actual2_min = actual2.min();
  let actual2_max = actual2.max();
  assert_eq!(mem::size_of_val(&actual2_min.x), mem::size_of_val(&15_i32));
  assert_eq!(mem::size_of_val(&actual2_min.y), mem::size_of_val(&25_i32));
  assert_eq!(mem::size_of_val(&actual2_max.x), mem::size_of_val(&35_i32));
  assert_eq!(mem::size_of_val(&actual2_max.y), mem::size_of_val(&45_i32));

  let r3: Rect<u32> = rect!(78_u32, 88_u32, 99_u32, 100_u32);
  let actual3 = rect_as!(r3, i16);
  let actual3_min = actual3.min();
  let actual3_max = actual3.max();
  assert_eq!(mem::size_of_val(&actual3_min.x), mem::size_of_val(&78_i16));
  assert_eq!(mem::size_of_val(&actual3_min.y), mem::size_of_val(&88_i16));
  assert_eq!(mem::size_of_val(&actual3_max.x), mem::size_of_val(&99_i16));
  assert_eq!(mem::size_of_val(&actual3_max.y), mem::size_of_val(&100_i16));
}

#[test]
fn cast_geo_sizes() {
  let s1: ISize = size!(1, 2);
  let actual1 = size_as!(s1, u8);
  let actual1_w = actual1.width();
  let actual1_h = actual1.height();
  assert_eq!(mem::size_of_val(&actual1_w), mem::size_of_val(&1_u8));
  assert_eq!(mem::size_of_val(&actual1_h), mem::size_of_val(&2_u8));

  let s2: U16Size = size!(15_u16, 25_u16);
  let actual2 = size_as!(s2, i32);
  let actual2_w = actual2.width();
  let actual2_h = actual2.height();
  assert_eq!(mem::size_of_val(&actual2_w), mem::size_of_val(&15_i32));
  assert_eq!(mem::size_of_val(&actual2_h), mem::size_of_val(&25_i32));

  let s3: Size<u32> = size!(78_u32, 88_u32);
  let actual3 = size_as!(s3, i16);
  let actual3_h = actual3.height();
  let actual3_w = actual3.width();
  assert_eq!(mem::size_of_val(&actual3_w), mem::size_of_val(&78_i16));
  assert_eq!(mem::size_of_val(&actual3_h), mem::size_of_val(&88_i16));
}

#[test]
fn cast_geo_size_into_rect() {
  let s1: ISize = size!(1, 2);
  let actual = rect_from_size!(s1);
  let actual = rect_as!(actual, u8);
  assert_eq!(mem::size_of_val(&actual.min().x), mem::size_of_val(&1_u8));
  assert_eq!(mem::size_of_val(&actual.min().y), mem::size_of_val(&1_u8));
  assert_eq!(mem::size_of_val(&actual.max().x), mem::size_of_val(&1_u8));
  assert_eq!(mem::size_of_val(&actual.max().y), mem::size_of_val(&1_u8));
  assert_eq!(actual.min().x, 0_u8);
  assert_eq!(actual.min().y, 0_u8);
  assert_eq!(actual.max().x, 1_u8);
  assert_eq!(actual.max().y, 2_u8);
}
