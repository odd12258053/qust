const DELIMITER: &u8 = &b' ';

pub(crate) fn is_delimiter(b: &u8) -> bool {
    b == DELIMITER
}

pub(crate) fn compare<T: Eq>(a: &[T], b: &[T]) -> Option<usize> {
    let mut aiter = a.iter();
    let mut biter = b.iter();

    let s = match aiter.next() {
        Some(s) => s,
        None => return None,
    };

    let o = match biter.next() {
        Some(o) => o,
        None => return None,
    };

    if s != o {
        return None;
    }
    let mut index = 0;
    loop {
        let s = match aiter.next() {
            Some(s) => s,
            None => break,
        };

        let o = match biter.next() {
            Some(o) => o,
            None => break,
        };
        if s != o {
            break;
        }
        index += 1;
    }
    Some(index)
}
