use crate::{error::FindFireError, firepoint::FirePoint};

use std::{error::Error, ffi::CString, path::Path};

use chrono::naive::NaiveDateTime;
use gdal::{raster::Buffer, Dataset};
use gdal_sys::{GDALAccess::GA_ReadOnly, GDALDatasetH};

static START: std::sync::Once = std::sync::Once::new();

pub struct FireSatImage {
    dataset: Dataset,
}

impl FireSatImage {
    /**
     * Parse the file name and find the scan start time.
     */
    pub fn find_start_time(fname: &str) -> Result<NaiveDateTime, FindFireError> {
        if let Some(i) = fname.find("_s") {
            let start = i + 2;
            let end = start + 13;
            let date_str = &fname[start..end];

            match NaiveDateTime::parse_from_str(date_str, "%Y%j%H%M%S") {
                Ok(st) => Ok(st),
                Err(_) => Err(FindFireError {
                    msg: "error parsing start time from file",
                }),
            }
        } else {
            Err(FindFireError {
                msg: "invalid filename format",
            })
        }
    }

    /**
     * Parse the file name and find the scan end time.
     */
    pub(crate) fn find_end_time(fname: &str) -> Result<NaiveDateTime, FindFireError> {
        if let Some(i) = fname.find("_e") {
            let start = i + 2;
            let end = start + 13;
            let date_str = &fname[start..end];

            match NaiveDateTime::parse_from_str(date_str, "%Y%j%H%M%S") {
                Ok(st) => Ok(st),
                Err(_) => Err(FindFireError {
                    msg: "error parsing start time from file",
                }),
            }
        } else {
            Err(FindFireError {
                msg: "invalid filename format",
            })
        }
    }
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let pth: &Path = path.as_ref();

        let dataset = unsafe {
            // Trick to get the library to call GDALAllRegister
            START.call_once(|| {
                let driver = gdal::Driver::get("netCDF");
                assert!(driver.is_ok());
            });

            let open_path = format!("NETCDF:\"{}\":Power", pth.to_string_lossy());
            let open_path: CString = CString::new(open_path)?;
            let c_handle: GDALDatasetH = gdal_sys::GDALOpen(open_path.as_ptr(), GA_ReadOnly);

            if c_handle.is_null() {
                Err(FindFireError {
                    msg: "error opening netcdf file",
                })
            } else {
                Ok(Dataset::from_c_dataset(c_handle))
            }
        }?;

        Ok(FireSatImage { dataset })
    }

    pub fn extract_fire_points(&self) -> Result<Vec<FirePoint>, Box<dyn Error>> {
        let mut points = vec![];

        let src_srs = self.dataset.spatial_ref()?;
        let dst_srs = gdal::spatial_ref::SpatialRef::from_epsg(4326)?;
        let trans = gdal::spatial_ref::CoordTransform::new(&src_srs, &dst_srs)?;
        let gtrans = self.dataset.geo_transform()?;

        let rasterband = self.dataset.rasterband(1)?;
        let Buffer {
            data,
            size: (x_size, y_size),
        } = rasterband.read_band_as::<f64>()?;

        assert_eq!(x_size, rasterband.x_size());
        assert_eq!(y_size, rasterband.y_size());

        for j in 0..y_size {
            for i in 0..x_size {
                let power = data[j * x_size + i];
                if power > 0.0 {
                    let mut xp: [f64; 1] =
                        [gtrans[0] + gtrans[1] * i as f64 + j as f64 * gtrans[2]];
                    let mut yp: [f64; 1] =
                        [gtrans[3] + gtrans[4] * i as f64 + j as f64 * gtrans[5]];
                    let mut zp: [f64; 1] = [0.0];

                    let _ = trans.transform_coords(&mut xp, &mut yp, &mut zp);

                    points.push(FirePoint {
                        lat: xp[0],
                        lon: yp[0],
                        power,
                        x: i as isize,
                        y: j as isize,
                    });
                }
            }
        }

        Ok(points)
    }
}
