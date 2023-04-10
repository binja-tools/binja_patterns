use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    ops::{Deref, Range},
    pin::Pin,
    slice,
    sync::Mutex,
};

use binaryninja::{
    binaryview::{BinaryView, BinaryViewBase, BinaryViewExt},
    interaction::get_text_line_input,
};
use log::{error, info};
use patterns::{Pattern, Scanner};
use static_init::dynamic;

#[dynamic]
pub static META: Mutex<HashMap<u64, FindState>> = Mutex::new(HashMap::new());

pub fn create_pattern(view: &BinaryView, range: Range<u64>) {
    let len = (range.end - range.start) as usize;
    let len = if len > 64 { 64 } else { len };
    let range = range.start..range.start + len as u64;

    let mut data = vec![0u8; len];
    if read_data(view, &mut data, range.clone()).is_err() {
        return;
    }
    let mut out: Vec<_> = data.iter().copied().map(Some).collect();

    let mut addr = range.start;
    while range.contains(&addr) {
        let data_addr = (addr - range.start) as usize;
        let data = &data[data_addr..];
        let out = &mut out[data_addr..];

        let Some(mut offset) = instruction_pattern(view, data, addr, out) else {
            return;
        };
        if offset == 0 {
            offset = 1;
        }
        addr += offset;
    }

    let pattern: String = out
        .into_iter()
        .map(|x| x.map(|x| format!("{x:02x} ")).unwrap_or("? ".to_string()))
        .collect();

    info!("Full Pattern:");
    info!("{}", &pattern[..pattern.len()]);

    let mut data = vec![0u8; view.len()];
    load_binary(view, &mut data);

    let Ok(pat) = pattern.parse() else {
        error!("cannot parse pattern!");
        return;
    };
    let pat: Pattern = pat;
    if pat.matches(&data).take(2).count() > 1 {
        return;
    }

    let pats: Vec<_> = pattern.split_ascii_whitespace().collect();

    let (mut left, _) = pats.split_at(pats.len() / 2);
    let mut l_bound = 0;
    let mut r_bound = pats.len();

    loop {
        let pat = left.join(" ");
        if let Ok(pattern) = pat.parse() {
            let pattern: Pattern = pattern;

            if pattern.matches(&data).take(2).count() > 1 {
                if (r_bound - l_bound) / 2 <= 1 {
                    let pivot = l_bound + (r_bound - l_bound) / 2;
                    (left, _) = pats.split_at(pivot + 1);
                    break;
                }

                l_bound = left.len();
                let pivot = l_bound + (r_bound - l_bound) / 2;
                (left, _) = pats.split_at(pivot);
            } else {
                if (r_bound - l_bound) / 2 <= 1 {
                    break;
                }
                r_bound = left.len();
                let pivot = l_bound + (r_bound - l_bound) / 2;
                (left, _) = pats.split_at(pivot);
            }
        }
    }

    let pattern = left.join(" ");

    info!("Shortest Unique:");
    info!("{}", &pattern[..pattern.len()]);
}

fn instruction_pattern(
    view: &BinaryView,
    data: &[u8],
    addr: u64,
    out: &mut [Option<u8>],
) -> Option<u64> {
    use binaryninjacore_sys::*;
    let Some(arch) = view.default_arch() else {
        error!("no default arch found");
        return None;
    };
    let functions = view.functions();
    let mut function = None;
    for func in functions.iter() {
        if (func.start()..func.highest_address()).contains(&addr) {
            function = Some(func);
        }
    }
    let Some(func) = function else {
        error!("range start outside of function");
        return None;
    };
    let Some(mut len) = view.instruction_len(&arch, addr) else {
        error!("no instruction length found");
        return None;
    };
    if len > data.len() {
        len = data.len();
    }

    let data = &data[..len];
    let out = &mut out[..len];
    let consts = unsafe {
        let func: *mut BNFunction = std::mem::transmute(func);
        let arch: *mut BNArchitecture = std::mem::transmute(arch);
        let mut count = 0_usize;
        let consts: *mut BNConstantReference =
            BNGetConstantsReferencedByInstruction(func, arch, addr, &mut count as *mut usize);
        let consts: &mut [BNConstantReference] = slice::from_raw_parts_mut(consts, count);
        consts
    };
    let mut offset = len;
    for con in consts.iter() {
        let d1 = (offset >= 1)
            .then(|| data.get(offset - 1).map(|&x| x as i64))
            .flatten();
        let d4 = (offset >= 4)
            .then(|| {
                data.get(offset - 4..offset)
                    .map(|x| i32::from_le_bytes(x.try_into().unwrap()) as i64)
            })
            .flatten();
        offset -= match (con.pointer, Some(con.value)) {
            (true, _) => 4,
            (_, x) if x == d1 => 1,
            (_, x) if x == d4 => 4,
            _ => 0,
        };
    }
    unsafe {
        BNFreeConstantReferenceList(consts.as_mut_ptr());
    }
    if offset < len {
        out[offset..].fill(None);
    }
    Some(len as u64)
}

pub fn find_pattern(view: &BinaryView) {
    let Some(pattern) = get_text_line_input("Enter pattern. Wildcards allowed:    ?    ??    .    ..", "Find Pattern") else {
        return;
    };

    let pattern: Pattern = match pattern.parse() {
        Ok(pattern) => pattern,
        Err(e) => {
            error!("Invalid Pattern: {:?}", e);
            return;
        }
    };

    let pattern = Box::pin(pattern);
    let data = Pin::new(vec![0u8; view.len()]);

    let mut state = create_state(pattern, data);
    let data = &mut state.data;

    load_binary(view, data);

    find_and_navigate(view, state);
}

fn load_binary(view: &BinaryView, data: &mut [u8]) {
    let sections = view.sections();
    let sections = sections.iter().map(|x| x.address_range());

    for section in sections {
        let binary_start = section.start - view.start();
        let binary_end = section.end - view.start();

        let buf = unsafe { data.get_unchecked_mut(binary_start as usize..binary_end as usize) };

        if read_data(view, buf, section).is_err() {
            return;
        }
    }
}

fn read_data(view: &BinaryView, buf: &mut [u8], addr: Range<u64>) -> Result<(), ()> {
    let read = view.read(buf, addr.start);
    if read != buf.len() {
        error!("Couldn't read data: {read}/{}", buf.len());
        return Err(());
    };
    Ok(())
}

fn create_state(pattern: Pin<Box<Pattern>>, data: Pin<Vec<u8>>) -> FindState {
    let scanner = unsafe {
        Pattern::matches(
            &*(pattern.deref() as *const _),
            &*(data.deref() as *const _),
        )
    };

    FindState {
        pattern,
        data,
        scanner,
    }
}

pub fn find_next(view: &BinaryView) {
    let state = { META.lock().unwrap().remove(&get_hash(view)).unwrap() };

    find_and_navigate(view, state)
}

fn find_and_navigate(view: &BinaryView, mut state: FindState) {
    if let Some(found) = state.scanner.next() {
        {
            META.lock().unwrap().insert(get_hash(view), state);
        }

        let jmp = view.start() + found as u64;
        info!("Offset: {found:#x}");
        info!("Jmp location: {jmp:#x}");
        if view
            .file()
            .navigate_to(view.file().current_view(), jmp)
            .is_err()
        {
            error!("Unable to jump to location in current view");
        }
    } else {
        info!("Pattern not found");
    }
}

pub fn get_hash(view: &BinaryView) -> u64 {
    let mut hasher = DefaultHasher::new();
    view.hash(&mut hasher);
    hasher.finish()
}

pub struct FindState {
    #[allow(unused)]
    pattern: Pin<Box<Pattern>>,
    #[allow(unused)]
    data: Pin<Vec<u8>>,
    scanner: Scanner<'static, 'static, 'static>,
}
