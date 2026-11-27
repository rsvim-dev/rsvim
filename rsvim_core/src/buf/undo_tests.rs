use super::undo::*;
use crate::buf::opt::EndOfLineOption;
use crate::cli::CliOptions;
use crate::prelude::*;
use crate::state::ops as state_ops;
use crate::tests::evloop::*;
use crate::tests::log::init as test_log_init;
use compact_str::CompactString;
use compact_str::ToCompactString;
use ropey::Rope;
use ropey::RopeBuilder;
use std::ops::Range;
use std::time::Duration;

const MAX_SIZE: usize = 100;

#[cfg(test)]
mod tests_undo {
  use super::*;

  fn assert_insert(undo: &Undo, op_idx: usize, op: Insert) {
    assert!(undo.current().records().len() > op_idx);
    let actual = undo.current().records()[op_idx].clone();
    assert!(matches!(actual.op, Operation::Insert(_)));
    match actual.op {
      Operation::Insert(insert) => assert_eq!(insert, op),
      _ => unreachable!(),
    }
  }

  fn assert_delete(undo: &Undo, op_idx: usize, op: Delete) {
    assert!(undo.current().records().len() > op_idx);
    let actual = undo.current().records()[op_idx].clone();
    assert!(matches!(actual.op, Operation::Delete(_)));
    match actual.op {
      Operation::Delete(delete) => assert_eq!(delete, op),
      _ => unreachable!(),
    }
  }

  fn assert_rope(rope: &Rope, range: Range<usize>, expect: &str) {
    let chars = rope.chars_at(range.start);
    assert!(chars.len() >= range.end - range.start);
    let actual = chars
      .take(range.end - range.start)
      .collect::<CompactString>();
    assert_eq!(actual, expect.to_compact_string());
  }

  #[test]
  fn insert1() {
    let mut undo = Undo::new(MAX_SIZE);
    let payload = "Hello, World!";
    for (i, c) in payload.chars().enumerate() {
      undo.current_mut().insert(Insert {
        payload: c.to_compact_string(),
        start_char: i,
        end_char: i + c.to_compact_string().chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      });
    }
    let actual = undo.current();
    assert_eq!(actual.records().len(), 1);
    assert_insert(
      &undo,
      0,
      Insert {
        payload: payload.to_compact_string(),
        start_char: 0,
        end_char: payload.to_compact_string().chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );
    undo.commit();

    let actual = undo.current();
    assert!(actual.records().is_empty());
  }

  #[test]
  fn insert2() {
    let mut undo = Undo::new(MAX_SIZE);
    let payload1 = "Hello, ";
    for (i, c) in payload1.chars().enumerate() {
      undo.current_mut().insert(Insert {
        payload: c.to_compact_string(),
        start_char: i,
        end_char: i + 1,
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      });
    }
    let actual = undo.current();
    assert_eq!(actual.records().len(), 1);
    assert_insert(
      &undo,
      0,
      Insert {
        payload: payload1.to_compact_string(),
        start_char: 0,
        end_char: payload1.to_compact_string().chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );

    let payload2 = "World!";
    for (i, c) in payload2.chars().enumerate() {
      undo.current_mut().insert(Insert {
        start_char: i + 3,
        end_char: i + 4,
        payload: c.to_compact_string(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      });
    }
    let actual = undo.current();
    assert_eq!(actual.records().len(), 2);
    assert_insert(
      &undo,
      0,
      Insert {
        payload: "Hello, ".to_compact_string(),
        start_char: 0,
        end_char: "Hello, ".chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );
    assert_insert(
      &undo,
      1,
      Insert {
        payload: payload2.to_compact_string(),
        start_char: 3,
        end_char: 3 + "World!".chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );

    let payload3 = "汤姆(Tom)?";
    for (i, c) in payload3.chars().enumerate() {
      undo.current_mut().insert(Insert {
        start_char: i + payload1.chars().count() + payload2.chars().count(),
        end_char: i
          + payload1.chars().count()
          + payload2.chars().count()
          + c.to_compact_string().chars().count(),
        payload: c.to_compact_string(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      });
    }
    let actual = undo.current();
    assert_eq!(actual.records().len(), 3);
    assert_insert(
      &undo,
      0,
      Insert {
        payload: payload1.to_compact_string(),
        start_char: 0,
        end_char: payload1.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );
    assert_insert(
      &undo,
      1,
      Insert {
        payload: payload2.to_compact_string(),
        start_char: 3,
        end_char: 3 + payload2.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );
    assert_insert(
      &undo,
      2,
      Insert {
        payload: payload3.to_compact_string(),
        start_char: payload1.chars().count() + payload2.chars().count(),
        end_char: payload1.chars().count()
          + payload2.chars().count()
          + payload3.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );

    let payload4 = "no, it's jerry";
    for (i, c) in payload4.chars().enumerate() {
      undo.current_mut().insert(Insert {
        payload: c.to_compact_string(),
        start_char: i + 100,
        end_char: i + 100 + 1,
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      });
    }
    let actual = undo.current();
    assert_eq!(actual.records().len(), 4);
    assert_insert(
      &undo,
      0,
      Insert {
        payload: payload1.to_compact_string(),
        start_char: 0,
        end_char: payload1.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );
    assert_insert(
      &undo,
      1,
      Insert {
        payload: payload2.to_compact_string(),
        start_char: 3,
        end_char: 3 + payload2.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );
    assert_insert(
      &undo,
      2,
      Insert {
        payload: payload3.to_compact_string(),
        start_char: payload1.chars().count() + payload2.chars().count(),
        end_char: payload1.chars().count()
          + payload2.chars().count()
          + payload3.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );
    assert_insert(
      &undo,
      3,
      Insert {
        payload: payload4.to_compact_string(),
        start_char: 100,
        end_char: 100 + payload4.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );

    undo.commit();

    let actual = undo.current();
    assert!(actual.records().is_empty());
  }

  #[test]
  fn delete1() {
    let mut undo = Undo::new(MAX_SIZE);
    let payload1 = "Hello, World!";
    for (i, c) in payload1.chars().enumerate() {
      undo.current_mut().insert(Insert {
        payload: c.to_compact_string(),
        start_char: i,
        end_char: i + 1,
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      });
    }

    let actual = undo.current();
    assert_eq!(actual.records().len(), 1);
    assert_insert(
      &undo,
      0,
      Insert {
        payload: payload1.to_compact_string(),
        start_char: 0,
        end_char: payload1.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );

    undo.current_mut().delete(Delete {
      payload: "!".to_compact_string(),
      start_char: 12,
      end_char: 13,
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });

    let actual = undo.current();
    assert_eq!(actual.records().len(), 2);
    assert_insert(
      &undo,
      0,
      Insert {
        payload: payload1.to_compact_string(),
        start_char: 0,
        end_char: payload1.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );
    assert_delete(
      &undo,
      1,
      Delete {
        payload: "!".to_compact_string(),
        start_char: 12,
        end_char: 13,
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );

    let payload2 = "Tom（汤姆） and Jerry（杰瑞）。";
    undo.current_mut().insert(Insert {
      payload: payload2.to_compact_string(),
      start_char: 12,
      end_char: 12 + payload2.chars().count(),
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });

    let actual = undo.current();
    assert_eq!(actual.records().len(), 3);
    assert_insert(
      &undo,
      0,
      Insert {
        payload: payload1.to_compact_string(),
        start_char: 0,
        end_char: payload1.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );
    assert_delete(
      &undo,
      1,
      Delete {
        payload: "!".to_compact_string(),
        start_char: 12,
        end_char: 13,
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );
    assert_insert(
      &undo,
      2,
      Insert {
        payload: payload2.to_compact_string(),
        start_char: 12,
        end_char: 12 + payload2.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );

    undo.current_mut().delete(Delete {
      payload: payload2.to_compact_string(),
      start_char: 12,
      end_char: 12 + payload2.chars().count(),
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });

    let actual = undo.current();
    assert_eq!(actual.records().len(), 2);

    assert_insert(
      &undo,
      0,
      Insert {
        payload: payload1.to_compact_string(),
        start_char: 0,
        end_char: payload1.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );
    assert_delete(
      &undo,
      1,
      Delete {
        payload: "!".to_compact_string(),
        start_char: 12,
        end_char: 13,
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );

    undo.commit();

    let actual = undo.current();
    assert!(actual.records().is_empty());
  }

  #[test]
  fn delete2() {
    let mut undo = Undo::new(MAX_SIZE);
    let payload1 = "Hello, World!";
    for (i, c) in payload1.chars().enumerate() {
      undo.current_mut().insert(Insert {
        payload: c.to_compact_string(),
        start_char: i,
        end_char: i + 1,
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      });
    }

    let actual = undo.current();
    assert_eq!(actual.records().len(), 1);
    assert_insert(
      &undo,
      0,
      Insert {
        payload: payload1.to_compact_string(),
        start_char: 0,
        end_char: payload1.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );

    undo.current_mut().delete(Delete {
      payload: "!".to_compact_string(),
      start_char: 12,
      end_char: 13,
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });
    undo.current_mut().delete(Delete {
      payload: "d".to_compact_string(),
      start_char: 11,
      end_char: 12,
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });
    undo.current_mut().delete(Delete {
      payload: "l".to_compact_string(),
      start_char: 10,
      end_char: 11,
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });

    let actual = undo.current();
    assert_eq!(actual.records().len(), 2);

    assert_insert(
      &undo,
      0,
      Insert {
        payload: payload1.to_compact_string(),
        start_char: 0,
        end_char: payload1.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );
    assert_delete(
      &undo,
      1,
      Delete {
        payload: "ld!".to_compact_string(),
        start_char: 10,
        end_char: 13,
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );

    undo.current_mut().delete(Delete {
      payload: "or".to_compact_string(),
      start_char: 8,
      end_char: 10,
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });

    let actual = undo.current();
    assert_eq!(actual.records().len(), 2);

    assert_insert(
      &undo,
      0,
      Insert {
        payload: payload1.to_compact_string(),
        start_char: 0,
        end_char: payload1.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );
    assert_delete(
      &undo,
      1,
      Delete {
        payload: "orld!".to_compact_string(),
        start_char: 8,
        end_char: 13,
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );

    undo.commit();

    let actual = undo.current();
    assert!(actual.records().is_empty());
  }

  #[test]
  fn delete3() {
    let mut undo = Undo::new(MAX_SIZE);
    let payload1 = "Hello, World!";
    undo.current_mut().insert(Insert {
      payload: payload1.to_compact_string(),
      start_char: 0,
      end_char: payload1.chars().count(),
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });

    let actual = undo.current();
    assert_eq!(actual.records().len(), 1);
    assert_insert(
      &undo,
      0,
      Insert {
        payload: payload1.to_compact_string(),
        start_char: 0,
        end_char: payload1.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );

    undo.current_mut().delete(Delete {
      payload: ", ".to_compact_string(),
      start_char: 5,
      end_char: 7,
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });

    let actual = undo.current();
    assert_eq!(actual.records().len(), 2);

    assert_insert(
      &undo,
      0,
      Insert {
        payload: payload1.to_compact_string(),
        start_char: 0,
        end_char: payload1.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );
    assert_delete(
      &undo,
      1,
      Delete {
        payload: ", ".to_compact_string(),
        start_char: 5,
        end_char: 7,
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );

    undo.current_mut().delete(Delete {
      payload: "loWo".to_compact_string(),
      start_char: 3,
      end_char: 7,
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });

    let actual = undo.current();
    assert_eq!(actual.records().len(), 3);

    assert_insert(
      &undo,
      0,
      Insert {
        payload: payload1.to_compact_string(),
        start_char: 0,
        end_char: payload1.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );
    assert_delete(
      &undo,
      1,
      Delete {
        payload: ", ".to_compact_string(),
        start_char: 5,
        end_char: 7,
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );
    assert_delete(
      &undo,
      2,
      Delete {
        payload: "loWo".to_compact_string(),
        start_char: 3,
        end_char: 7,
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );
  }

  #[test]
  fn delete4() {
    let mut undo = Undo::new(MAX_SIZE);
    let payload1 = "Hello, World!";
    undo.current_mut().insert(Insert {
      payload: payload1.to_compact_string(),
      start_char: 0,
      end_char: payload1.chars().count(),
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });

    let actual = undo.current();
    assert_eq!(actual.records().len(), 1);
    assert_insert(
      &undo,
      0,
      Insert {
        payload: payload1.to_compact_string(),
        start_char: 0,
        end_char: payload1.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );

    undo.current_mut().delete(Delete {
      payload: "o".to_compact_string(),
      start_char: 8,
      end_char: 9,
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });
    undo.current_mut().delete(Delete {
      payload: "r".to_compact_string(),
      start_char: 9,
      end_char: 10,
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });

    let actual = undo.current();
    assert_eq!(actual.records().len(), 2);

    assert_insert(
      &undo,
      0,
      Insert {
        payload: payload1.to_compact_string(),
        start_char: 0,
        end_char: payload1.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );
    assert_delete(
      &undo,
      1,
      Delete {
        payload: "or".to_compact_string(),
        start_char: 8,
        end_char: 10,
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );

    undo.current_mut().delete(Delete {
      payload: "l".to_compact_string(),
      start_char: 10,
      end_char: 11,
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });
    undo.current_mut().delete(Delete {
      payload: "d".to_compact_string(),
      start_char: 11,
      end_char: 12,
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });
    undo.current_mut().delete(Delete {
      payload: "!".to_compact_string(),
      start_char: 12,
      end_char: 13,
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });

    let actual = undo.current();
    assert_eq!(actual.records().len(), 2);

    assert_insert(
      &undo,
      0,
      Insert {
        payload: payload1.to_compact_string(),
        start_char: 0,
        end_char: payload1.chars().count(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );
    assert_delete(
      &undo,
      1,
      Delete {
        payload: "orld!".to_compact_string(),
        start_char: 8,
        end_char: 13,
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      },
    );

    undo.commit();

    let actual = undo.current();
    assert!(actual.records().is_empty());
  }

  #[test]
  fn revert1() {
    test_log_init();

    let mut undo = Undo::new(MAX_SIZE);
    let mut text1 = RopeBuilder::new().finish();

    let payload1 = "Hello";
    for (i, c) in payload1.chars().enumerate() {
      text1.insert_char(i, c);
      undo.current_mut().insert(Insert {
        start_char: i,
        end_char: i + 1,
        payload: c.to_compact_string(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      });
    }

    let payload2 = ", ";
    text1.insert(payload1.len(), payload2);
    undo.current_mut().insert(Insert {
      start_char: payload1.len(),
      end_char: payload1.len() + payload2.len(),
      payload: payload2.to_compact_string(),
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });

    let payload3 = "World!";
    text1.insert(payload1.len() + payload2.len(), payload3);
    undo.current_mut().insert(Insert {
      start_char: payload1.len() + payload2.len(),
      end_char: payload1.len() + payload2.len() + payload3.len(),
      payload: payload3.to_compact_string(),
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });

    undo.commit();
    info!("undo:{:?}", undo);

    let mut text2 = text1.clone();
    let result = undo.undo(0, &mut text2);
    assert!(result.is_ok());
    assert_eq!(text2.len_chars(), 0);

    assert!(undo.undo_stack().is_empty());
    assert_eq!(undo.redo_stack().len(), 1);
  }

  #[test]
  fn revert2() {
    test_log_init();

    let mut undo = Undo::new(MAX_SIZE);
    let mut text1 = RopeBuilder::new().finish();

    let payload1 = "Hello, ";
    for (i, c) in payload1.chars().enumerate() {
      text1.insert_char(i, c);
      undo.current_mut().insert(Insert {
        start_char: i,
        end_char: i + 1,
        payload: c.to_compact_string(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      });
    }

    let payload2 = ", ";
    assert_rope(&text1, 5..7, payload2);
    text1.remove(5..7);
    undo.current_mut().delete(Delete {
      start_char: 5,
      end_char: 7,
      payload: payload2.to_compact_string(),
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });

    let payload3 = "World!";
    assert_eq!(payload1.len() - payload2.len(), 5);
    text1.insert(5, payload3);
    undo.current_mut().insert(Insert {
      start_char: 5,
      end_char: 5 + payload3.len(),
      payload: payload3.to_compact_string(),
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });

    let payload4 = "!";
    assert_rope(&text1, 10..11, payload4);
    text1.remove(10..11);
    undo.current_mut().delete(Delete {
      start_char: 10,
      end_char: 11,
      payload: payload4.to_compact_string(),
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });

    undo.commit();
    info!("undo:{:?}", undo);

    let mut text2 = text1.clone();

    assert_eq!(text1.to_compact_string(), "HelloWorld");
    assert_eq!(text2.to_compact_string(), "HelloWorld");
    assert_eq!(undo.undo_stack().len(), 4);
    assert_eq!(undo.redo_stack().len(), 0);

    let result1 = undo.undo(3, &mut text2);
    assert!(result1.is_ok());
    assert_eq!(text2.len_chars(), 11);
    assert_eq!(text2.to_compact_string(), "HelloWorld!");
    assert_eq!(undo.undo_stack().len(), 3);
    assert_eq!(undo.redo_stack().len(), 1);

    let result2 = undo.undo(2, &mut text2);
    assert!(result2.is_ok());
    assert_eq!(text2.len_chars(), 5);
    assert_eq!(text2.to_compact_string(), "Hello");
    assert_eq!(undo.undo_stack().len(), 2);
    assert_eq!(undo.redo_stack().len(), 2);

    let result3 = undo.undo(1, &mut text2);
    assert!(result3.is_ok());
    assert_eq!(text2.len_chars(), 7);
    assert_eq!(text2.to_compact_string(), "Hello, ");
    assert_eq!(undo.undo_stack().len(), 1);
    assert_eq!(undo.redo_stack().len(), 3);

    let result4 = undo.undo(0, &mut text2);
    assert!(result4.is_ok());
    assert_eq!(text2.len_chars(), 0);
    assert_eq!(text2.to_compact_string(), "");
    assert_eq!(undo.undo_stack().len(), 0);
    assert_eq!(undo.redo_stack().len(), 4);
  }

  #[test]
  fn revert3() {
    test_log_init();

    let mut undo = Undo::new(MAX_SIZE);
    let mut text1 = RopeBuilder::new().finish();

    let payload1 = "Hello, ";
    for (i, c) in payload1.chars().enumerate() {
      text1.insert_char(i, c);
      undo.current_mut().insert(Insert {
        start_char: i,
        end_char: i + 1,
        payload: c.to_compact_string(),
        cursor_char_idx_before: 0,
        cursor_char_idx_after: 0,
      });
    }

    let payload2 = ", ";
    assert_rope(&text1, 5..7, payload2);
    text1.remove(5..7);
    undo.current_mut().delete(Delete {
      start_char: 5,
      end_char: 7,
      payload: payload2.to_compact_string(),
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });

    let payload3 = "World!";
    assert_eq!(payload1.len() - payload2.len(), 5);
    text1.insert(5, payload3);
    undo.current_mut().insert(Insert {
      start_char: 5,
      end_char: 5 + payload3.len(),
      payload: payload3.to_compact_string(),
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });

    let payload4 = "!";
    assert_rope(&text1, 10..11, payload4);
    text1.remove(10..11);
    undo.current_mut().delete(Delete {
      start_char: 10,
      end_char: 11,
      payload: payload4.to_compact_string(),
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });

    undo.commit();
    info!("undo:{:?}", undo);

    let mut text2 = text1.clone();

    assert_eq!(text1.to_compact_string(), "HelloWorld");
    assert_eq!(text2.to_compact_string(), "HelloWorld");
    assert_eq!(undo.undo_stack().len(), 4);
    assert_eq!(undo.redo_stack().len(), 0);

    let result1 = undo.undo(3, &mut text2);
    assert!(result1.is_ok());
    assert_eq!(text2.len_chars(), 11);
    assert_eq!(text2.to_compact_string(), "HelloWorld!");
    assert_eq!(undo.undo_stack().len(), 3);
    assert_eq!(undo.redo_stack().len(), 1);

    let result2 = undo.undo(2, &mut text2);
    assert!(result2.is_ok());
    assert_eq!(text2.len_chars(), 5);
    assert_eq!(text2.to_compact_string(), "Hello");
    assert_eq!(undo.undo_stack().len(), 2);
    assert_eq!(undo.redo_stack().len(), 2);

    let result3 = undo.undo(1, &mut text2);
    assert!(result3.is_ok());
    assert_eq!(text2.len_chars(), 7);
    assert_eq!(text2.to_compact_string(), "Hello, ");
    assert_eq!(undo.undo_stack().len(), 1);
    assert_eq!(undo.redo_stack().len(), 3);

    let result4 = undo.undo(0, &mut text2);
    assert!(result4.is_ok());
    assert_eq!(text2.len_chars(), 0);
    assert_eq!(text2.to_compact_string(), "");
    assert_eq!(undo.undo_stack().len(), 0);
    assert_eq!(undo.redo_stack().len(), 4);
  }

  #[test]
  fn revert4() {
    test_log_init();

    let mut undo = Undo::new(MAX_SIZE);
    let mut text1 = RopeBuilder::new().finish();

    let payload1 = "Hello";
    text1.insert(0, payload1);
    undo.current_mut().insert(Insert {
      start_char: 0,
      end_char: payload1.chars().count(),
      payload: payload1.to_compact_string(),
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });
    undo.commit();

    let payload2 = ", ";
    text1.insert(5, payload2);
    undo.current_mut().insert(Insert {
      start_char: 5,
      end_char: 5 + payload2.chars().count(),
      payload: payload2.to_compact_string(),
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });
    undo.commit();

    let payload3 = "World";
    text1.insert(7, payload3);
    undo.current_mut().insert(Insert {
      start_char: 7,
      end_char: 7 + payload3.len(),
      payload: payload3.to_compact_string(),
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });
    undo.commit();

    let payload4 = ", ";
    assert_rope(&text1, 5..7, payload4);
    text1.remove(5..7);
    undo.current_mut().delete(Delete {
      start_char: 5,
      end_char: 7,
      payload: payload4.to_compact_string(),
      cursor_char_idx_before: 0,
      cursor_char_idx_after: 0,
    });
    undo.commit();

    info!("undo:{:?}", undo);

    let mut text2 = text1.clone();

    assert_eq!(text1.to_compact_string(), "HelloWorld");
    assert_eq!(text2.to_compact_string(), "HelloWorld");
    assert_eq!(undo.undo_stack().len(), 4);
    assert_eq!(undo.redo_stack().len(), 0);

    let result1 = undo.undo(3, &mut text2);
    assert!(result1.is_ok());
    assert_eq!(text2.len_chars(), 12);
    assert_eq!(text2.to_compact_string(), "Hello, World");
    assert_eq!(undo.undo_stack().len(), 3);
    assert_eq!(undo.redo_stack().len(), 1);

    let result2 = undo.undo(2, &mut text2);
    assert!(result2.is_ok());
    assert_eq!(text2.len_chars(), 7);
    assert_eq!(text2.to_compact_string(), "Hello, ");
    assert_eq!(undo.undo_stack().len(), 2);
    assert_eq!(undo.redo_stack().len(), 2);

    let result3 = undo.undo(1, &mut text2);
    assert!(result3.is_ok());
    assert_eq!(text2.len_chars(), 5);
    assert_eq!(text2.to_compact_string(), "Hello");
    assert_eq!(undo.undo_stack().len(), 1);
    assert_eq!(undo.redo_stack().len(), 3);

    let result4 = undo.undo(0, &mut text2);
    assert!(result4.is_ok());
    assert_eq!(text2.len_chars(), 0);
    assert_eq!(text2.to_compact_string(), "");
    assert_eq!(undo.undo_stack().len(), 0);
    assert_eq!(undo.redo_stack().len(), 4);
  }
}

#[cfg(test)]
mod tests_buffer_editing {
  use super::*;

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn insert1() -> IoResult<()> {
    test_log_init();

    let src: &str = r#""#;

    // Prepare $RSVIM_CONFIG/rsvim.js
    let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

    let terminal_cols = 10_u16;
    let terminal_rows = 10_u16;
    let mocked_ops = vec![
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Keep,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("Hello".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text(", ".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("World".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(30)),
    ];

    let mut event_loop =
      make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

    event_loop.initialize()?;
    event_loop
      .run_with_mock_operations(MockOperationReader::new(mocked_ops))
      .await?;
    event_loop.shutdown()?;

    // After running
    {
      let buf = lock!(event_loop.editor.buffer_manager)
        .first_key_value()
        .unwrap()
        .1
        .clone();
      let mut buf = lock!(buf);
      let buf_eol = Into::<EndOfLineOption>::into(buf.options().file_format());
      let after_payload = buf.text().rope().to_string();
      assert_eq!(after_payload, format!("Hello, World{}", buf_eol));
      let max_commits = buf.undo().undo_stack().len();
      debug_assert_eq!(max_commits, 1);
      let mut revert_rope = buf.text().rope().clone();
      buf.undo_mut().undo(0, &mut revert_rope).unwrap();
      let before_payload = revert_rope.to_string();
      assert_eq!(before_payload, format!("{}", buf_eol));
    }

    Ok(())
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn delete1() -> IoResult<()> {
    test_log_init();

    let src: &str = r#""#;

    // Prepare $RSVIM_CONFIG/rsvim.js
    let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

    let terminal_cols = 10_u16;
    let terminal_rows = 10_u16;
    let mocked_ops = vec![
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Keep,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("Hello".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Append,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text(", ".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Append,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("World".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      // Hello, World
      MockOperation::Operation(state_ops::Operation::CursorMoveTo((7, 0))),
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Keep,
      )),
      MockOperation::Operation(state_ops::Operation::CursorDelete(-2)),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(30)),
    ];

    let mut event_loop =
      make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

    event_loop.initialize()?;
    event_loop
      .run_with_mock_operations(MockOperationReader::new(mocked_ops))
      .await?;
    event_loop.shutdown()?;

    // After running
    {
      let buf = lock!(event_loop.editor.buffer_manager)
        .first_key_value()
        .unwrap()
        .1
        .clone();
      let mut buf = lock!(buf);
      info!("undo_stack:{:?}", buf.undo().undo_stack());

      debug_assert_eq!(buf.undo().undo_stack().len(), 4);
      let buf_eol = Into::<EndOfLineOption>::into(buf.options().file_format());
      let after_payload = buf.text().rope().to_string();
      assert_eq!(after_payload, format!("HelloWorld{}", buf_eol));
      let mut rope = buf.text().rope().clone();

      {
        buf.undo_mut().undo(3, &mut rope).unwrap();
        let before_payload1 = rope.to_string();
        assert_eq!(before_payload1, format!("Hello, World{}", buf_eol));
      }

      {
        debug_assert_eq!(buf.undo().undo_stack().len(), 3);
        buf.undo_mut().undo(2, &mut rope).unwrap();
        let before_payload2 = rope.to_string();
        assert_eq!(before_payload2, format!("Hello, {}", buf_eol));
      }

      {
        debug_assert_eq!(buf.undo().undo_stack().len(), 2);
        buf.undo_mut().undo(1, &mut rope).unwrap();
        let before_payload3 = rope.to_string();
        assert_eq!(before_payload3, format!("Hello{}", buf_eol));
      }

      {
        debug_assert_eq!(buf.undo().undo_stack().len(), 1);
        buf.undo_mut().undo(0, &mut rope).unwrap();
        let before_payload4 = rope.to_string();
        assert_eq!(before_payload4, format!("{}", buf_eol));
      }
    }

    Ok(())
  }
}
