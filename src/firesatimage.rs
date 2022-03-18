use crate::{
    geo::Coord,
    pixel::Pixel,
    satellite::{DataQualityFlagCode, MaskCode},
    SatFireResult,
};
use libc::{c_char, c_double, c_int, c_short, c_void, size_t};
use once_cell::sync::OnceCell;
use std::{
    ffi::{CStr, CString},
    io::Read,
    path::Path,
    sync::Mutex,
};

static_assertions::assert_eq_size!(c_short, i16);
static_assertions::assert_eq_size!(c_double, f64);

/**
 * Handle to a dataset for the Fire Detection Characteristics and some metadata.
 */
#[derive(Debug, Clone)]
pub(crate) struct SatFireImage {
    /// Image width in pixels
    xlen: usize,
    /// Image height in pixels
    ylen: usize,
    /// All the information needed for transforming from row and column numbers to coordinates.
    tran: CoordTransform,
    /// In memory buffer if this is from a zip file
    buffer: Option<Vec<u8>>,
    /// Handle to the NetCDF file
    nc_file_id: c_int,
    /// Orignial file name the dataset was loaded from.
    fname: String,
}

macro_rules! check_error {
    ($code:expr) => {
        check_netcdf_error($code, file!(), line!())
    };
    ($code:expr, "attr") => {
        check_netcdf_attribute_error($code, file!(), line!())
    };
}

impl SatFireImage {
    /// Open a file containing GOES-R/S Fire Detection Characteristics.
    pub(crate) fn open<P: AsRef<Path>>(path: P) -> SatFireResult<Self> {
        let p: &Path = path.as_ref();
        // FIXME change option into error
        let fname: String = p
            .file_name()
            .map(|p| p.to_string_lossy())
            .unwrap()
            .to_string();

        if let Some(ext) = p.extension() {
            if ext == "zip" {
                Self::open_zip(p, fname)
            } else if ext == "nc" {
                Self::open_nc(p, fname)
            } else {
                Err(std::io::Error::from(std::io::ErrorKind::Unsupported).into())
            }
        } else {
            Err(std::io::Error::from(std::io::ErrorKind::InvalidInput).into())
        }
    }

    fn open_zip(p: &Path, fname: String) -> SatFireResult<Self> {
        let path_str = CString::new(p.to_string_lossy().as_bytes())?;

        let file = std::fs::File::open(p)?;
        let mut zip = zip::ZipArchive::new(file)?;
        assert_eq!(zip.len(), 1);

        let mut nc_file = zip.by_index(0)?;
        let mut buf: Vec<u8> = Vec::with_capacity(nc_file.size() as usize + 10);
        let _size_read = nc_file.read_to_end(&mut buf)?;

        let lock = get_netcdf_lock()
            .lock()
            .expect("Error locking global mutex for netCDF");
        let mut file_id: c_int = -1;
        unsafe {
            let status = nc_open_mem(
                path_str.as_ptr(),
                NC_NOWRITE,
                buf.len(),
                buf.as_mut_ptr() as *mut c_void,
                &mut file_id as *mut c_int,
            );
            if status != NC_NOERR {
                return Err(format!(
                    "Error opening netcdf: {}",
                    std::str::from_utf8_unchecked(CStr::from_ptr(nc_strerror(status)).to_bytes())
                )
                .into());
            }
        }

        let res = Self::initialize_with_nc_file_handle(fname, file_id, Some(buf))?;

        drop(lock);

        Ok(res)
    }

    fn open_nc(p: &Path, fname: String) -> SatFireResult<Self> {
        let path_str = CString::new(p.to_string_lossy().as_bytes())?;

        let lock = get_netcdf_lock()
            .lock()
            .expect("Error locking global mutex for netCDF");
        let mut file_id: c_int = -1;
        unsafe {
            let status = nc_open(path_str.as_ptr(), NC_NOWRITE, &mut file_id as *mut c_int);
            check_error!(status)?;
        }

        let res = Self::initialize_with_nc_file_handle(fname, file_id, None)?;

        drop(lock);

        Ok(res)
    }

    #[allow(non_snake_case)]
    fn initialize_with_nc_file_handle(
        fname: String,
        handle: c_int,
        in_memory_buffer: Option<Vec<u8>>,
    ) -> SatFireResult<Self> {
        let mut xlen: usize = 0;
        let mut ylen: usize = 0;

        // Shorthand
        let h = handle;

        let mut xscale: f64 = f64::NAN;
        let mut xoffset: f64 = f64::NAN;
        let mut yscale: f64 = f64::NAN;
        let mut yoffset: f64 = f64::NAN;
        let mut req: f64 = f64::NAN;
        let mut rpol: f64 = f64::NAN;
        let mut H: f64 = f64::NAN;
        let mut lon0: f64 = f64::NAN;

        unsafe {
            let mut xdimid: c_int = -1;
            let mut status = nc_inq_dimid(
                h,
                b"x\0".as_ptr() as *const c_char,
                &mut xdimid as *mut c_int,
            );
            check_error!(status)?;
            status = nc_inq_dimlen(h, xdimid, &mut xlen as *mut size_t);
            check_error!(status)?;

            let mut ydimid: c_int = -1;
            status = nc_inq_dimid(
                h,
                b"y\0".as_ptr() as *const c_char,
                &mut ydimid as *mut c_int,
            );
            check_error!(status)?;
            status = nc_inq_dimlen(h, ydimid, &mut ylen as *mut size_t);
            check_error!(status)?;

            let mut x: c_int = -1;
            let mut y: c_int = -1;
            status = nc_inq_varid(h, b"x\0".as_ptr() as *const c_char, &mut x as *mut c_int);
            check_error!(status)?;
            status = nc_inq_varid(h, b"y\0".as_ptr() as *const c_char, &mut y as *mut c_int);
            check_error!(status)?;

            let scale_factor = b"scale_factor\0".as_ptr() as *const c_char;
            status = nc_get_att_double(h, x, scale_factor, &mut xscale as *mut c_double);
            check_error!(status)?;
            status = nc_get_att_double(h, y, scale_factor, &mut yscale as *mut c_double);
            check_error!(status)?;

            let add_offset = b"add_offset\0".as_ptr() as *const c_char;
            status = nc_get_att_double(h, x, add_offset, &mut xoffset as *mut c_double);
            check_error!(status)?;
            status = nc_get_att_double(h, y, add_offset, &mut yoffset as *mut c_double);
            check_error!(status)?;

            let mut proj_id: c_int = -1;
            status = nc_inq_varid(
                h,
                b"goes_imager_projection\0".as_ptr() as *const c_char,
                &mut proj_id as *mut c_int,
            );
            check_error!(status)?;

            let semi_major_axis = b"semi_major_axis\0".as_ptr() as *const c_char;
            let semi_minor_axis = b"semi_minor_axis\0".as_ptr() as *const c_char;
            let perp_point_h = b"perspective_point_height\0".as_ptr() as *const c_char;
            let lon_origin = b"longitude_of_projection_origin\0".as_ptr() as *const c_char;
            status = nc_get_att_double(h, proj_id, semi_major_axis, &mut req as *mut c_double);
            check_error!(status)?;
            status = nc_get_att_double(h, proj_id, semi_minor_axis, &mut rpol as *mut c_double);
            check_error!(status)?;
            status = nc_get_att_double(h, proj_id, perp_point_h, &mut H as *mut c_double);
            check_error!(status)?;
            status = nc_get_att_double(h, proj_id, lon_origin, &mut lon0 as *mut c_double);
            check_error!(status)?;
        }

        Ok(SatFireImage {
            xlen,
            ylen,
            tran: CoordTransform {
                xscale,
                xoffset,
                yscale,
                yoffset,
                req,
                rpol,
                H: H + req,
                lon0,
            },
            buffer: in_memory_buffer,
            nc_file_id: handle,
            fname,
        })
    }

    pub(crate) fn extract_fire_points(&self) -> SatFireResult<Vec<FirePoint>> {
        let mut points: Vec<FirePoint> = Vec::new();

        let lock = get_netcdf_lock()
            .lock()
            .expect("Error locking global mutex for netCDF");

        let powers = self.extract_variable_double(b"Power\0".as_ptr() as *const c_char)?;
        let areas = self.extract_variable_double(b"Area\0".as_ptr() as *const c_char)?;
        let temperatures = self.extract_variable_double(b"Temp\0".as_ptr() as *const c_char)?;
        let masks = self.extract_variable_short(b"Mask\0".as_ptr() as *const c_char)?;
        let dqfs = self.extract_variable_short(b"DQF\0".as_ptr() as *const c_char)?;

        drop(lock);

        for j in 0..self.ylen {
            for i in 0..self.xlen {
                let index = i + j * self.xlen;

                let power_mw;
                let area;
                let temperature;
                let mask;
                let dqf;

                unsafe {
                    power_mw = *powers.get_unchecked(index);
                    area = *areas.get_unchecked(index);
                    temperature = *temperatures.get_unchecked(index);
                    mask = *masks.get_unchecked(index);
                    dqf = *dqfs.get_unchecked(index);
                }

                // 0 for a data quality flag indicates a good quality fire detection
                if dqf == 0 {
                    let ii = i as f64;
                    let jj = j as f64;

                    let ips: [f64; 5] = [ii - 0.5, ii - 0.5, ii + 0.5, ii + 0.5, ii];
                    let jps: [f64; 5] = [jj - 0.5, jj + 0.5, jj + 0.5, jj - 0.5, jj];

                    let (scan_angle, coords) = self.tran.convert_row_cols_to_latlon(&jps, &ips);

                    points.push(FirePoint {
                        x: i as isize,
                        y: j as isize,
                        pixel: Pixel {
                            ul: coords[0],
                            ll: coords[1],
                            lr: coords[2],
                            ur: coords[3],
                            power: power_mw,
                            area,
                            temperature,
                            mask_flag: MaskCode(mask),
                            data_quality_flag: DataQualityFlagCode(dqf),
                            scan_angle,
                        },
                    });
                }
            }
        }

        Ok(points)
    }

    fn extract_variable_double(&self, vname: *const c_char) -> SatFireResult<Vec<f64>> {
        let mut vals = Vec::with_capacity(self.xlen * self.ylen);

        let mut skip_transform;
        let mut scale_factor: f64 = 1.0;
        let mut add_offset: f64 = 0.0;
        let mut fill_value: f64 = 65535.0;

        unsafe {
            let fid = self.nc_file_id;
            let mut varid: c_int = -1;
            let mut status = nc_inq_varid(fid, vname, &mut varid as *mut c_int);
            check_error!(status)?;

            let start: [size_t; 2] = [0, 0];
            let counts: [size_t; 2] = [self.ylen, self.xlen];
            let (start, counts) = (start.as_ptr(), counts.as_ptr());
            status = nc_get_vara_double(fid, varid, start, counts, vals.as_mut_ptr());
            check_error!(status)?;

            vals.set_len(self.ylen * self.xlen);

            let scale_str = b"scale_factor\0".as_ptr() as *const c_char;
            let offset_str = b"add_offset\0".as_ptr() as *const c_char;
            let fill_str = b"_FillValue\0".as_ptr() as *const c_char;
            status = nc_get_att_double(fid, varid, scale_str, &mut scale_factor as *mut c_double);
            check_error!(status, "attr")?;
            skip_transform = status == NC_ENOTATT;
            status = nc_get_att_double(fid, varid, offset_str, &mut add_offset as *mut c_double);
            check_error!(status, "attr")?;
            skip_transform = skip_transform && (status == NC_ENOTATT);
            status = nc_get_att_double(fid, varid, fill_str, &mut fill_value as *mut c_double);
            check_error!(status, "attr")?;
        }

        if skip_transform {
            for val in vals.iter_mut() {
                *val = if *val == fill_value {
                    -f64::INFINITY
                } else {
                    *val * scale_factor + add_offset
                };
            }
        } else {
            for val in vals.iter_mut() {
                if *val == fill_value {
                    *val = -f64::INFINITY;
                }
            }
        }

        Ok(vals)
    }

    fn extract_variable_short(&self, vname: *const c_char) -> SatFireResult<Vec<i16>> {
        let mut vals = Vec::with_capacity(self.xlen * self.ylen);

        unsafe {
            let mut varid: c_int = -1;
            let mut status = nc_inq_varid(self.nc_file_id, vname, &mut varid as *mut c_int);
            check_error!(status)?;

            let start: [size_t; 2] = [0, 0];
            let counts: [size_t; 2] = [self.ylen, self.xlen];

            status = nc_get_vara_short(
                self.nc_file_id,
                varid,
                start.as_ptr(),
                counts.as_ptr(),
                vals.as_mut_ptr(),
            );
            check_error!(status)?;

            vals.set_len(self.ylen * self.xlen);
        }

        Ok(vals)
    }
}

impl Drop for SatFireImage {
    fn drop(&mut self) {
        let lock = get_netcdf_lock()
            .lock()
            .expect("Error locking global mutex for netCDF");

        unsafe {
            let _ = nc_close(self.nc_file_id);
        }

        drop(lock);
    }
}

/**
 * Represents all the data associated with a single pixel in which the satellite has detected
 * a fire.
 */
#[derive(Debug, Clone, Copy)]
pub(crate) struct FirePoint {
    /// The polygon describing the scanned area.
    pub pixel: Pixel,
    /// The x-coordinate (column number, often indexed as 'i') in the grid.
    pub x: isize,
    /// The y-coordinate (row number, often indexed as 'j') in the grid.
    pub y: isize,
}

/// Projection information required to convert from row/column number to scan angles and lat-lon.
#[allow(non_snake_case)]
#[derive(Debug, Clone, Copy)]
struct CoordTransform {
    /// Scale factor for the column for converting indexes to scan angle coords.
    xscale: f64,
    /// Offset for the column for converting indexes to scan angle coords
    xoffset: f64,
    /// Scale factor for the row for converting indexes to scan angle coords
    yscale: f64,
    /// Offset for the  row for converting indexes to scan angle coords
    yoffset: f64,
    /// Radius of the Earth at the equator in meters.
    req: f64,
    /// Radius of the Earth at the poles in meters.
    rpol: f64,
    /// Height of the satellite above the equator in meters.
    H: f64,
    /// Longitude of the nadir point in degrees.
    lon0: f64,
}

impl CoordTransform {
    #[allow(non_snake_case)]
    fn convert_row_cols_to_latlon(&self, rows: &[f64; 5], cols: &[f64; 5]) -> (f64, [Coord; 5]) {
        let mut coords = [Coord { lat: 0.0, lon: 0.0 }; 5];

        let x = self.xscale * cols[4] + self.xoffset;
        let y = self.yscale * rows[4] + self.yoffset;
        let scan_angle = x.hypot(y).to_degrees();

        for i in 0..5 {
            let x = self.xscale * cols[i] + self.xoffset;
            let y = self.yscale * rows[i] + self.yoffset;

            let sinx = x.sin();
            let cosx = x.cos();
            let siny = y.sin();
            let cosy = y.cos();
            let req = self.req;
            let rpol = self.rpol;
            let H = self.H;
            let lon0 = self.lon0;

            let a =
                sinx * sinx + cosx * cosx * (cosy * cosy + req * req / (rpol * rpol) * siny * siny);
            let b = -2.0 * H * cosx * cosy;
            let c = H * H - req * req;

            let rs = (-b - (b * b - 4.0 * a * c).sqrt()) / (2.0 * a);

            let sx = rs * cosx * cosy;
            let sy = -rs * sinx;
            let sz = rs * cosx * siny;

            let lat = (req * req * sz)
                .atan2(rpol * rpol * ((H - sx) * (H - sx) + sy * sy).sqrt())
                .to_degrees();
            let lon = lon0 - (sy.atan2(H - sx)).to_degrees();

            coords[i] = Coord { lat, lon };
        }

        (scan_angle, coords)
    }
}

static NETCDF_GLOBAL_LOCK: OnceCell<Mutex<()>> = OnceCell::new();

fn get_netcdf_lock() -> &'static Mutex<()> {
    NETCDF_GLOBAL_LOCK.get_or_init(|| Mutex::new(()))
}

const NC_NOWRITE: c_int = 0x0000;
const NC_NOERR: c_int = 0;
const NC_ENOTATT: c_int = -43;

fn check_netcdf_error(status_code: c_int, file: &'static str, line: u32) -> SatFireResult<()> {
    unsafe {
        if status_code != NC_NOERR {
            Err(format!(
                "{}[{}]netCDF error: {}",
                file,
                line,
                std::str::from_utf8_unchecked(CStr::from_ptr(nc_strerror(status_code)).to_bytes())
            )
            .into())
        } else {
            Ok(())
        }
    }
}

fn check_netcdf_attribute_error(
    status_code: c_int,
    file: &'static str,
    line: u32,
) -> SatFireResult<()> {
    unsafe {
        if status_code != NC_NOERR && status_code != NC_ENOTATT {
            Err(format!(
                "{}[{}]netCDF error loading attribute: {}",
                file,
                line,
                std::str::from_utf8_unchecked(CStr::from_ptr(nc_strerror(status_code)).to_bytes())
            )
            .into())
        } else {
            Ok(())
        }
    }
}

#[link(name = "netcdf")]
extern "C" {
    fn nc_open(path: *const c_char, mode: c_int, ncidp: *mut c_int) -> c_int;
    fn nc_open_mem(
        name: *const c_char,
        mode: c_int,
        buf_size: size_t,
        buf: *mut c_void,
        ncidp: *mut c_int,
    ) -> c_int;
    fn nc_close(handle: c_int) -> c_int;

    fn nc_strerror(code: c_int) -> *const c_char;

    fn nc_inq_dimid(handle: c_int, name: *const c_char, rv: *mut c_int) -> c_int;
    fn nc_inq_dimlen(handle: c_int, dimid: c_int, rv: *mut size_t) -> c_int;
    fn nc_inq_varid(handle: c_int, name: *const c_char, varid: *mut c_int) -> c_int;
    fn nc_get_att_double(
        handle: c_int,
        varid: c_int,
        name: *const c_char,
        val: *mut c_double,
    ) -> c_int;
    fn nc_get_vara_short(
        handle: c_int,
        varid: c_int,
        start: *const size_t,
        counts: *const size_t,
        vals: *mut c_short,
    ) -> c_int;
    fn nc_get_vara_double(
        handle: c_int,
        varid: c_int,
        start: *const size_t,
        counts: *const size_t,
        vals: *mut c_double,
    ) -> c_int;
}
