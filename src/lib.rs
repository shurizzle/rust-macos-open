//! Cose

extern crate core_foundation;
extern crate core_foundation_sys;
extern crate fast_escape;
#[macro_use]
extern crate fast_fmt;
extern crate launch_services;
extern crate void;

use core_foundation::array::CFArray;
use core_foundation::base::TCFType;
use core_foundation::string::{CFString, CFStringRef};
use core_foundation::url::{CFURLRef, CFURL};
use core_foundation_sys::base::{kCFAllocatorDefault, CFAllocatorRef};
use launch_services::{
    application_urls_for_bundle_identifier, application_urls_for_url, can_url_accept_url,
    default_application_url_for_url, open_from_url_spec, open_url,
    LSAcceptanceFlags, LSLaunchURLSpec, LSRolesMask,
};

pub use launch_services::LSLaunchFlags;

use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};

pub use core_foundation_sys::base::OSStatus;

use fast_escape::Escaper;
use file_metadata::mditem::attributes;
use file_metadata::mdquery::{MDQuery, MDQueryOptionFlags};
use void::ResultVoidExt;

#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    fn CFURLCreateWithString(
        allocator: CFAllocatorRef,
        urlString: CFStringRef,
        baseURL: CFURLRef,
    ) -> CFURLRef;
}

/// Trait
pub trait Openable {
    fn into_openable(&self) -> Option<CFURL>;
}

fn url(value: &str) -> Option<CFURL> {
    let url = CFString::new(value);
    let ptr = unsafe {
        CFURLCreateWithString(
            kCFAllocatorDefault,
            url.as_concrete_TypeRef(),
            std::ptr::null(),
        )
    };

    if ptr.is_null() {
        None
    } else {
        Some(unsafe { TCFType::wrap_under_create_rule(ptr) })
    }
}

impl Openable for &str {
    fn into_openable(&self) -> Option<CFURL> {
        url(self)
    }
}

impl Openable for str {
    fn into_openable(&self) -> Option<CFURL> {
        url(self)
    }
}

impl Openable for &String {
    fn into_openable(&self) -> Option<CFURL> {
        url(self)
    }
}

impl Openable for String {
    fn into_openable(&self) -> Option<CFURL> {
        url(self)
    }
}

impl Openable for &Path {
    fn into_openable(&self) -> Option<CFURL> {
        Openable::into_openable(self.to_str()?)
    }
}

impl Openable for Path {
    fn into_openable(&self) -> Option<CFURL> {
        Openable::into_openable(self.to_str()?)
    }
}

impl Openable for &PathBuf {
    fn into_openable(&self) -> Option<CFURL> {
        Openable::into_openable(self.to_str()?)
    }
}

impl Openable for PathBuf {
    fn into_openable(&self) -> Option<CFURL> {
        Openable::into_openable(self.to_str()?)
    }
}

/// Trait
pub trait MultiOpenable {
    fn into_openable(&self) -> Option<CFArray<CFURL>>;
}

macro_rules! def_multiopenable_vec {
    ( $type:ty ) => {
        impl MultiOpenable for Vec<$type> {
            fn into_openable(&self) -> Option<CFArray<CFURL>> {
                let mut res: Vec<CFURL> = Vec::new();

                for el in self.iter() {
                    match Openable::into_openable(el) {
                        None => return None,
                        Some(url) => res.push(url),
                    };
                }

                Some(CFArray::<CFURL>::from_CFTypes(&res[..]))
            }
        }

        impl MultiOpenable for &[$type] {
            fn into_openable(&self) -> Option<CFArray<CFURL>> {
                let mut res: Vec<CFURL> = Vec::new();

                for el in self.iter() {
                    match Openable::into_openable(el) {
                        None => return None,
                        Some(url) => res.push(url),
                    };
                }

                Some(CFArray::<CFURL>::from_CFTypes(&res[..]))
            }
        }

        impl MultiOpenable for [$type] {
            fn into_openable(&self) -> Option<CFArray<CFURL>> {
                let mut res: Vec<CFURL> = Vec::new();

                for el in self.iter() {
                    match Openable::into_openable(el) {
                        None => return None,
                        Some(url) => res.push(url),
                    };
                }

                Some(CFArray::<CFURL>::from_CFTypes(&res[..]))
            }
        }
    };
}

macro_rules! def_multiopenable_type {
    ( $type:ty ) => {
        impl MultiOpenable for $type {
            fn into_openable(&self) -> Option<CFArray<CFURL>> {
                let v = vec![Openable::into_openable(self)?];
                Some(CFArray::<CFURL>::from_CFTypes(&v[..]))
            }
        }
    };
}

macro_rules! def_multiopenable {
    ( $type:ty ) => {
        def_multiopenable_vec!($type);
        def_multiopenable_type!($type);
    };
}

def_multiopenable!(&str);
def_multiopenable_vec!(&String);
def_multiopenable_type!(str);
def_multiopenable!(String);
def_multiopenable!(&Path);
def_multiopenable_vec!(&PathBuf);
def_multiopenable_type!(Path);
def_multiopenable!(PathBuf);

pub fn open<T: Openable + ?Sized>(url: &T) -> Result<Option<PathBuf>> {
    if let Some(openable) = Openable::into_openable(url) {
        match open_url(&openable) {
            Ok(path) => Ok(path.to_path()),
            Err(code) => Err(Error::new(
                ErrorKind::Other,
                format!("return code {}", code),
            )),
        }
    } else {
        Err(Error::new(ErrorKind::Other, "Provided url is not openable"))
    }
}

#[inline]
fn remap_app(app: Option<&Path>) -> Result<Option<CFURL>> {
    if let Some(app) = app {
        match CFURL::from_path(app, true) {
            None => Err(Error::new(
                ErrorKind::Other,
                "Provided app url is not valid",
            )),
            res => Ok(res),
        }
    } else {
        Ok(None)
    }
}

#[inline]
fn remap_multiopenable<T: MultiOpenable + ?Sized>(
    urls: Option<&T>,
) -> Result<Option<CFArray<CFURL>>> {
    if let Some(urls) = urls {
        match MultiOpenable::into_openable(urls) {
            None => Err(Error::new(ErrorKind::Other, "Provided urls are not valid")),
            res => Ok(res),
        }
    } else {
        Ok(None)
    }
}

pub fn open_complex<T: MultiOpenable + ?Sized>(
    app: Option<&Path>,
    urls: Option<&T>,
    flags: LSLaunchFlags,
) -> Result<Option<PathBuf>> {
    let spec = LSLaunchURLSpec {
        app: remap_app(app)?,
        urls: remap_multiopenable(urls)?,
        flags,
        ..Default::default()
    };

    match open_from_url_spec(spec) {
        Ok(path) => Ok(path.to_path()),
        Err(code) => Err(Error::new(
            ErrorKind::Other,
            format!("return code {}", code),
        )),
    }
}

pub fn default_app_for_scheme(scheme: &str) -> Option<PathBuf> {
    let scheme = Openable::into_openable(&format!("{}://", scheme))?;
    match default_application_url_for_url(&scheme, LSRolesMask::VIEWER) {
        Ok(url) => url.to_path(),
        Err(_) => None,
    }
}

pub fn apps_for_scheme(scheme: &str) -> Option<Vec<PathBuf>> {
    let scheme = Openable::into_openable(&format!("{}://", scheme))?;
    Some(
        application_urls_for_url(&scheme, LSRolesMask::VIEWER)?
            .iter()
            .filter_map(|v| v.to_path())
            .collect::<Vec<_>>(),
    )
}

pub fn apps_for_bundle_id(bundle_id: &str) -> Option<Vec<PathBuf>> {
    let bundle_id = CFString::new(bundle_id);
    match application_urls_for_bundle_identifier(&bundle_id) {
        Ok(apps) => Some(apps.iter().filter_map(|v| v.to_path()).collect()),
        Err(_) => None,
    }
}

pub fn app_for_bundle_id(bundle_id: &str) -> Option<PathBuf> {
    let mut apps = apps_for_bundle_id(bundle_id)?;
    if apps.is_empty() {
        None
    } else {
        Some(apps.remove(0))
    }
}

const MQ_STRING_SPECIAL_CHARS: [char; 4] = ['?', '*', '\\', '"'];

pub fn apps_for_name(app_name: &str) -> Option<Vec<PathBuf>> {
    let mut query_string = String::new();
    let escaper: Escaper<&[char]> = Escaper::new('\\', &MQ_STRING_SPECIAL_CHARS);
    fwrite!(
        &mut query_string,
        "kMDItemContentTypeTree == \"com.apple.application\"c && kMDItemDisplayName == \"",
        app_name.transformed(escaper),
        "\"cd"
    )
    .void_unwrap();
    let query_cfstring = CFString::new(&query_string);
    let query = MDQuery::new(query_cfstring, None, None)?;
    query.execute(MDQueryOptionFlags::SYNC | MDQueryOptionFlags::ALLOW_FS_TRANSLATION);
    query.stop();

    let res = query
        .iter()
        .filter_map(|v| {
            v.get(attributes::Path)
                .map(|a| PathBuf::from(a.to_string()))
        })
        .collect::<Vec<_>>();
    if res.len() == 0 {
        None
    } else {
        Some(res)
    }
}

pub fn app_for_name(name: &str) -> Option<PathBuf> {
    let mut apps = apps_for_name(name)?;
    if apps.is_empty() {
        None
    } else {
        Some(apps.remove(0))
    }
}

pub fn app_accept_urls<T: MultiOpenable + ?Sized>(app: &Path, urls: &T) -> bool {
    if let Some(app) = CFURL::from_path(app, true) {
        match MultiOpenable::into_openable(urls) {
            None => return false,
            Some(urls) => !urls
                .iter()
                .map(|v| {
                    match can_url_accept_url(
                        &*v,
                        &app,
                        LSRolesMask::VIEWER,
                        LSAcceptanceFlags::DEFAULT,
                    ) {
                        Err(_) => false,
                        Ok(res) => res,
                    }
                })
                .any(|v| !v),
        }
    } else {
        false
    }
}

pub fn apps_for_name_accepting_urls<T: MultiOpenable + ?Sized>(
    name: &str,
    urls: &T,
) -> Option<Vec<PathBuf>> {
    let res: Vec<PathBuf> = apps_for_name(name)?
        .into_iter()
        .filter(|v| app_accept_urls(v, urls))
        .collect();

    if res.is_empty() {
        None
    } else {
        Some(res)
    }
}

pub fn app_for_name_accepting_urls<T: MultiOpenable + ?Sized>(
    name: &str,
    urls: &T,
) -> Option<PathBuf> {
    let mut apps = apps_for_name_accepting_urls(name, urls)?;

    if apps.is_empty() {
        None
    } else {
        Some(apps.remove(0))
    }
}

pub fn apps_for_bundle_id_accepting_urls<T: MultiOpenable + ?Sized>(
    bundle_id: &str,
    urls: &T,
) -> Option<Vec<PathBuf>> {
    let res: Vec<PathBuf> = apps_for_bundle_id(bundle_id)?
        .into_iter()
        .filter(|v| app_accept_urls(v, urls))
        .collect();

    if res.is_empty() {
        None
    } else {
        Some(res)
    }
}

pub fn app_for_bundle_id_accepting_urls<T: MultiOpenable + ?Sized>(
    bundle_id: &str,
    urls: &T,
) -> Option<PathBuf> {
    let mut apps = apps_for_bundle_id_accepting_urls(bundle_id, urls)?;

    if apps.is_empty() {
        None
    } else {
        Some(apps.remove(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        open("https://www.google.com/").unwrap();

        open_complex(
            Some(Path::new("/Applications/Safari.app")),
            Some(&["https://news.ycombinator.com/", "https://www.google.com/"][..]),
            LSLaunchFlags::DEFAULTS,
        )
        .unwrap();

        println!("{:#?}", apps_for_bundle_id("com.google.chrome"));
        println!("{:#?}", app_for_bundle_id("com.google.chrome"));

        println!(
            "{:#?}",
            app_for_name_accepting_urls("Google Chrome", &["http://www.google.com/"][..])
        );
        println!(
            "{:#?}",
            app_for_bundle_id_accepting_urls("com.google.chrome", &["http://www.google.com/"][..])
        );
    }
}
