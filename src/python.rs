use std::pin::Pin;

use binaryninja::binary_view::{BinaryView, BinaryViewBase};
use patterns::{ParsePatternError, Pattern, Scanner};
use pyo3::{exceptions::PyValueError, prelude::*};

use crate::patterns::load_binary;

#[pyclass(frozen)]
struct BvPattern {
    // todo: consider making this a Ref<BV>
    #[allow(unused)]
    bv: BinaryView,
    data: Pin<Vec<u8>>,
}

#[pymethods]
impl BvPattern {
    #[new]
    fn new(bv: &Bound<'_, PyAny>) -> PyResult<Self> {
        let handle = bv.getattr("handle")?;
        let ctypes = bv.py().import("ctypes")?;
        let handle = ctypes
            .call_method1("cast", (handle, ctypes.getattr("c_void_p")?))?
            .getattr("value")?;

        let handle: u64 = handle.extract()?;
        let bv: BinaryView = unsafe { std::mem::transmute(handle) };

        let mut data = Pin::new(vec![0u8; bv.len() as usize]);
        load_binary(&bv, &mut data);

        Ok(Self { bv, data })
    }

    #[pyo3(signature = (pattern, /))]
    fn find(this: PyRef<'_, Self>, pattern: &str) -> PyResult<Search> {
        let pattern = Box::pin(Pattern::from_str(pattern).map_err(convert_err)?);
        let scanner = pattern.matches(this.data.as_ref().get_ref());

        Ok(Search {
            scanner: unsafe { std::mem::transmute(scanner) },
            pattern,
        })
    }
}

#[pyclass]
struct Search {
    scanner: Scanner<'static, 'static, 1, 64>,
    #[allow(unused)]
    pattern: Pin<Box<Pattern>>,
}

#[pymethods]
impl Search {
    fn __iter__(this: PyRef<Self>) -> PyRef<Self> {
        this
    }

    fn __next__(mut this: PyRefMut<Self>) -> Option<usize> {
        this.scanner.next()
    }
}

#[pymodule]
fn binja_patterns(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<BvPattern>()?;

    Ok(())
}

fn convert_err(e: ParsePatternError) -> PyErr {
    let s = match e {
        ParsePatternError::PatternTooLong => "pattern too long",
        ParsePatternError::InvalidHexNumber(_) => "invalid hex number",
        ParsePatternError::MissingNonWildcardByte => "missing non-wildcard byte",
        _ => "unknown error",
    };

    PyErr::new::<PyValueError, _>(s)
}
