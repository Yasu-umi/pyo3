// Copyright (c) 2017-present PyO3 Project and Contributors

//! Python Mapping Interface
//! Trait and support implementation for implementing mapping support

use std::os::raw::c_int;

use ffi;
use err::{PyErr, PyResult};
use pyptr::Py;
use python::Python;
use objects::{exc, PyObject};
use callback::{PyObjectCallbackConverter, LenResultConverter, UnitCallbackConverter};
use conversion::{ToPyObject, FromPyObject};
use typeob::PyTypeInfo;
use class::methods::PyMethodDef;


/// Mapping interface
#[allow(unused_variables)]
pub trait PyMappingProtocol<'p>: PyTypeInfo + Sized + 'static {

    fn __len__(&'p self, py: Python<'p>)
               -> Self::Result where Self: PyMappingLenProtocol<'p> {unimplemented!()}

    fn __getitem__(&'p self, py: Python<'p>, key: Self::Key)
                   -> Self::Result where Self: PyMappingGetItemProtocol<'p> {unimplemented!()}

    fn __setitem__(&'p self, py: Python<'p>, key: Self::Key, value: Self::Value)
                   -> Self::Result where Self: PyMappingSetItemProtocol<'p> {unimplemented!()}

    fn __delitem__(&'p self, py: Python<'p>, key: Self::Key)
                   -> Self::Result where Self: PyMappingDelItemProtocol<'p> {unimplemented!()}

    fn __iter__(&'p self, py: Python<'p>)
                -> Self::Result where Self: PyMappingIterProtocol<'p> {unimplemented!()}

    fn __contains__(&'p self, py: Python<'p>, value: Self::Value)
                    -> Self::Result where Self: PyMappingContainsProtocol<'p> {unimplemented!()}

    fn __reversed__(&'p self, py: Python<'p>)
                    -> Self::Result where Self: PyMappingReversedProtocol<'p> {unimplemented!()}

}


// The following are a bunch of marker traits used to detect
// the existance of a slotted method.

pub trait PyMappingLenProtocol<'p>: PyMappingProtocol<'p> {
    type Result: Into<PyResult<usize>>;
}

pub trait PyMappingGetItemProtocol<'p>: PyMappingProtocol<'p> {
    type Key: FromPyObject<'p>;
    type Success: ToPyObject;
    type Result: Into<PyResult<Self::Success>>;
}

pub trait PyMappingSetItemProtocol<'p>: PyMappingProtocol<'p> {
    type Key: FromPyObject<'p>;
    type Value: FromPyObject<'p>;
    type Result: Into<PyResult<()>>;
}

pub trait PyMappingDelItemProtocol<'p>: PyMappingProtocol<'p> {
    type Key: FromPyObject<'p>;
    type Result: Into<PyResult<()>>;
}

pub trait PyMappingIterProtocol<'p>: PyMappingProtocol<'p> {
    type Success: ToPyObject;
    type Result: Into<PyResult<Self::Success>>;
}

pub trait PyMappingContainsProtocol<'p>: PyMappingProtocol<'p> {
    type Value: FromPyObject<'p>;
    type Result: Into<PyResult<bool>>;
}

pub trait PyMappingReversedProtocol<'p>: PyMappingProtocol<'p> {
    type Success: ToPyObject;
    type Result: Into<PyResult<Self::Success>>;
}

#[doc(hidden)]
pub trait PyMappingProtocolImpl {
    fn tp_as_mapping() -> Option<ffi::PyMappingMethods>;
    fn methods() -> Vec<PyMethodDef>;
}

impl<T> PyMappingProtocolImpl for T {
    #[inline]
    default fn tp_as_mapping() -> Option<ffi::PyMappingMethods> {
        None
    }
    #[inline]
    default fn methods() -> Vec<PyMethodDef> {
        Vec::new()
    }
}

impl<'p, T> PyMappingProtocolImpl for T where T: PyMappingProtocol<'p> {
    #[inline]
    fn tp_as_mapping() -> Option<ffi::PyMappingMethods> {
        let f = if let Some(df) = Self::mp_del_subscript() {
            Some(df)
        } else {
            Self::mp_ass_subscript()
        };

        Some(ffi::PyMappingMethods {
            mp_length: Self::mp_length(),
            mp_subscript: Self::mp_subscript(),
            mp_ass_subscript: f,
        })
    }

    #[inline]
    fn methods() -> Vec<PyMethodDef> {
        let mut methods = Vec::new();

        if let Some(def) = <Self as PyMappingIterProtocolImpl>::__iter__() {
            methods.push(def)
        }
        if let Some(def) = <Self as PyMappingContainsProtocolImpl>::__contains__() {
            methods.push(def)
        }
        if let Some(def) = <Self as PyMappingReversedProtocolImpl>::__reversed__() {
            methods.push(def)
        }

        methods
    }
}

trait PyMappingLenProtocolImpl {
    fn mp_length() -> Option<ffi::lenfunc>;
}

impl<'p, T> PyMappingLenProtocolImpl for T where T: PyMappingProtocol<'p>
{
    #[inline]
    default fn mp_length() -> Option<ffi::lenfunc> {
        None
    }
}

impl<T> PyMappingLenProtocolImpl for T where T: for<'p> PyMappingLenProtocol<'p>
{
    #[inline]
    fn mp_length() -> Option<ffi::lenfunc> {
        py_len_func!(PyMappingLenProtocol, T::__len__, LenResultConverter)
    }
}

trait PyMappingGetItemProtocolImpl {
    fn mp_subscript() -> Option<ffi::binaryfunc>;
}

impl<'p, T> PyMappingGetItemProtocolImpl for T where T: PyMappingProtocol<'p>
{
    #[inline]
    default fn mp_subscript() -> Option<ffi::binaryfunc> {
        None
    }
}

impl<T> PyMappingGetItemProtocolImpl for T where T: for<'p> PyMappingGetItemProtocol<'p>
{
    #[inline]
    fn mp_subscript() -> Option<ffi::binaryfunc> {
        py_binary_func!(PyMappingGetItemProtocol,
                        T::__getitem__, T::Success, PyObjectCallbackConverter)
    }
}

trait PyMappingSetItemProtocolImpl {
    fn mp_ass_subscript() -> Option<ffi::objobjargproc>;
}

impl<'p, T> PyMappingSetItemProtocolImpl for T where T: PyMappingProtocol<'p>
{
    #[inline]
    default fn mp_ass_subscript() -> Option<ffi::objobjargproc> {
        None
    }
}

impl<T> PyMappingSetItemProtocolImpl for T where T: for<'p> PyMappingSetItemProtocol<'p>
{
    #[inline]
    fn mp_ass_subscript() -> Option<ffi::objobjargproc> {
        unsafe extern "C" fn wrap<T>(slf: *mut ffi::PyObject,
                                     key: *mut ffi::PyObject,
                                     value: *mut ffi::PyObject) -> c_int
            where T: for<'p> PyMappingSetItemProtocol<'p>
        {
            const LOCATION: &'static str = "T.__setitem__()";
            ::callback::cb_unary::<T, _, _, _>(LOCATION, slf, UnitCallbackConverter, |py, slf| {
                let res = if value.is_null() {
                    Err(PyErr::new::<exc::NotImplementedError, _>(
                        py, format!("Subscript deletion not supported by {:?}",
                                    stringify!(T))))
                } else {
                    let key = PyObject::from_borrowed_ptr(py, key);
                    match key.extract() {
                        Ok(key) => {
                            let value = PyObject::from_borrowed_ptr(py, value);
                            match value.extract() {
                                Ok(value) => slf.__setitem__(py, key, value).into(),
                                Err(e) => Err(e),
                            }
                        },
                        Err(e) => Err(e),
                    }
                };
                res
            })
        }
        Some(wrap::<T>)
    }
}


trait PyMappingDelItemProtocolImpl {
    fn mp_del_subscript() -> Option<ffi::objobjargproc>;
}

impl<'p, T> PyMappingDelItemProtocolImpl for T where T: PyMappingProtocol<'p>
{
    #[inline]
    default fn mp_del_subscript() -> Option<ffi::objobjargproc> {
        None
    }
}

impl<T> PyMappingDelItemProtocolImpl for T where T: for<'p> PyMappingDelItemProtocol<'p>
{
    #[inline]
    default fn mp_del_subscript() -> Option<ffi::objobjargproc> {
        unsafe extern "C" fn wrap<T>(slf: *mut ffi::PyObject,
                                     key: *mut ffi::PyObject,
                                     value: *mut ffi::PyObject) -> c_int
            where T: for<'p> PyMappingDelItemProtocol<'p>
        {
            const LOCATION: &'static str = "T.__detitem__()";
            ::callback::cb_unary::<T, _, _, _>(LOCATION, slf, UnitCallbackConverter, |py, slf| {
                if value.is_null() {
                    let key = PyObject::from_borrowed_ptr(py, key);
                    match key.extract() {
                        Ok(key) => {
                            slf.__delitem__(py, key).into()
                        },
                        Err(e) => Err(e),
                    }
                } else {
                    Err(PyErr::new::<exc::NotImplementedError, _>(
                        py, format!("Subscript assignment not supported by {:?}",
                                    stringify!(T))))
                }
            })
        }
        Some(wrap::<T>)
    }
}


impl<T> PyMappingDelItemProtocolImpl for T
    where T: for<'p> PyMappingSetItemProtocol<'p> + for<'p> PyMappingDelItemProtocol<'p>
{
    #[inline]
    fn mp_del_subscript() -> Option<ffi::objobjargproc> {
        unsafe extern "C" fn wrap<T>(slf: *mut ffi::PyObject,
                                     key: *mut ffi::PyObject,
                                     value: *mut ffi::PyObject) -> c_int
            where T: for<'p> PyMappingSetItemProtocol<'p> + for<'p> PyMappingDelItemProtocol<'p>
        {
            const LOCATION: &'static str = "T.__set/del_item__()";
            ::callback::handle(LOCATION, UnitCallbackConverter, |py| {
                let slf: Py<T> = Py::from_borrowed_ptr(py, slf);
                let key = PyObject::from_borrowed_ptr(py, key);

                if value.is_null() {
                    match key.extract() {
                        Ok(key) => slf.__delitem__(py, key).into(),
                        Err(e) => Err(e)
                    }
                } else {
                    match key.extract() {
                        Ok(key) => {
                            let value = PyObject::from_borrowed_ptr(py, value);
                            match value.extract() {
                                Ok(value) => slf.__setitem__(py, key, value).into(),
                                Err(e) => Err(e),
                            }
                        },
                        Err(e) => Err(e),
                    }
                }
            })
        }
        Some(wrap::<T>)
    }
}


#[doc(hidden)]
pub trait PyMappingContainsProtocolImpl {
    fn __contains__() -> Option<PyMethodDef>;
}

impl<'p, T> PyMappingContainsProtocolImpl for T where T: PyMappingProtocol<'p>
{
    #[inline]
    default fn __contains__() -> Option<PyMethodDef> {
        None
    }
}

#[doc(hidden)]
pub trait PyMappingReversedProtocolImpl {
    fn __reversed__() -> Option<PyMethodDef>;
}

impl<'p, T> PyMappingReversedProtocolImpl for T where T: PyMappingProtocol<'p>
{
    #[inline]
    default fn __reversed__() -> Option<PyMethodDef> {
        None
    }
}

#[doc(hidden)]
pub trait PyMappingIterProtocolImpl {
    fn __iter__() -> Option<PyMethodDef>;
}

impl<'p, T> PyMappingIterProtocolImpl for T where T: PyMappingProtocol<'p>
{
    #[inline]
    default fn __iter__() -> Option<PyMethodDef> {
        None
    }
}
