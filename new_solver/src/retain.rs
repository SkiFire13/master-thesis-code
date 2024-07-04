pub enum Simplify {
    Keep,
    Remove,
    Clear,
}

pub fn simplify<T>(v: &mut Vec<T>, mut f: impl FnMut(usize, usize, &mut T) -> Simplify) -> bool {
    let mut clear = false;
    let mut old_i = 0;
    let mut new_i = 0;

    v.retain_mut(|v| {
        if clear {
            return false;
        }

        let keep = match f(old_i, new_i, v) {
            Simplify::Keep => true,
            Simplify::Remove => false,
            Simplify::Clear => {
                clear = true;
                false
            }
        };

        old_i += 1;
        if keep {
            new_i += 1;
        }

        keep
    });

    clear
}
