//! Very simple functions for producing KML files specifcally suited to this crate and the programs
//! that use it.
//!
//! This is not a general solution at all, but I opted to create it instead of pulling another
//! potentially large dependency. I actually did test using the [KML](https://github.com/georust/kml)
//! crate. However, when generating large KML files, it crashed because it took too much memory. So
//! for this implementation I'm only implementing the parts I need with a focus on a more streaming
//! type API. That means the user is responsible for closing all tags.

use crate::SatFireResult;
use chrono::{DateTime, Utc};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

pub struct KmlFile(BufWriter<File>);

impl KmlFile {
    pub fn new<P: AsRef<Path>>(pth: P) -> SatFireResult<Self> {
        let p = pth.as_ref();

        let f = std::fs::File::create(p)?;
        let mut new = KmlFile(BufWriter::new(f));
        new.start_document()?;
        Ok(new)
    }
}

impl KmlWriter for KmlFile {
    fn output(&mut self) -> &mut dyn Write {
        &mut self.0
    }
}

impl Drop for KmlFile {
    fn drop(&mut self) {
        self.finish_document();
    }
}

pub trait KmlWriter {
    fn output(&mut self) -> &mut dyn Write;

    /// Open a file for output and start by putting the header out.
    fn start_document(&mut self) -> SatFireResult<()> {
        const HEADER: &str = concat!(
            r#"<?xml version="1.0" encoding="UTF-8"?>"#,
            "\n",
            r#"<kml xmlns="http://www.opengis.net/kml/2.2">"#,
            "\n",
            "<Document>\n"
        );

        self.output().write_all(HEADER.as_bytes())?;

        Ok(())
    }

    /// Close a document.
    fn finish_document(&mut self) {
        const FOOTER: &str = concat!(r#"</Document>"#, "\n", r#"</kml>"#, "\n");
        let _ = self.output().write_all(FOOTER.as_bytes());
    }

    /// Write a description element to the file.
    fn write_description(&mut self, description: &str) -> SatFireResult<()> {
        writeln!(
            self.output(),
            "<description><![CDATA[{}]]></description>",
            description
        )?;
        Ok(())
    }

    /// Start a KML folder.
    fn start_folder(
        &mut self,
        name: Option<&str>,
        description: Option<&str>,
        is_open: bool,
    ) -> SatFireResult<()> {
        self.output().write_all("<Folder>\n".as_bytes())?;

        if let Some(name) = name {
            writeln!(self.output(), "<name>{}</name>", name)?;
        }

        if let Some(description) = description {
            self.write_description(description)?;
        }

        if is_open {
            self.output().write_all("<open>1</open>\n".as_bytes())?;
        }

        Ok(())
    }

    /// Close out a folder element
    fn finish_folder(&mut self) -> SatFireResult<()> {
        writeln!(self.output(), "</Folder>")?;
        Ok(())
    }

    /// Start a placemark element.
    fn start_placemark(
        &mut self,
        name: Option<&str>,
        description: Option<&str>,
        style_url: Option<&str>,
    ) -> SatFireResult<()> {
        writeln!(self.output(), "<Placemark>")?;

        if let Some(name) = name {
            writeln!(self.output(), "<name>{}</name>", name)?;
        }

        if let Some(description) = description {
            self.write_description(description)?;
        }

        if let Some(style_url) = style_url {
            writeln!(self.output(), "<styleUrl>{}</styleUrl>", style_url)?;
        }

        Ok(())
    }

    /// Close out a placemark element.
    fn finish_placemark(&mut self) -> SatFireResult<()> {
        writeln!(self.output(), "</Placemark>")?;
        Ok(())
    }

    /// Start a style definition.
    fn start_style(&mut self, style_id: Option<&str>) -> SatFireResult<()> {
        if let Some(style_id) = style_id {
            writeln!(self.output(), "<Style id=\"{}\">", style_id)?;
        } else {
            writeln!(self.output(), "<Style>")?;
        }
        Ok(())
    }

    /// Close out a style definition.
    fn finish_style(&mut self) -> SatFireResult<()> {
        writeln!(self.output(), "</Style>")?;
        Ok(())
    }

    /// Create a PolyStyle element.
    ///
    /// These should ONLY go inside a style element.
    fn create_poly_style(
        &mut self,
        color: Option<&str>,
        filled: bool,
        outlined: bool,
    ) -> SatFireResult<()> {
        writeln!(self.output(), "<PolyStyle>")?;

        if let Some(color) = color {
            writeln!(self.output(), "<color>{}</color>", color)?;
            writeln!(self.output(), "<colorMode>normal</colorMode>")?;
        } else {
            writeln!(self.output(), "<colorMode>random</colorMode>")?;
        }

        let filled = if filled { 1 } else { 0 };
        let outlined = if outlined { 1 } else { 0 };

        writeln!(self.output(), "<fill>{}</fill>", filled)?;
        writeln!(self.output(), "<outline>{}</outline>", outlined)?;

        writeln!(self.output(), "</PolyStyle>")?;
        Ok(())
    }

    /// Create an IconStyle element.
    fn create_icon_style(&mut self, icon_url: Option<&str>, scale: f64) -> SatFireResult<()> {
        writeln!(self.output(), "<IconStyle>")?;

        if scale > 0.0 {
            writeln!(self.output(), "<scale>{}</scale>", scale)?;
        } else {
            writeln!(self.output(), "<scale>1</scale>")?;
        }

        if let Some(icon_url) = icon_url {
            writeln!(self.output(), "<Icon><href>{}</href></Icon>", icon_url)?;
        }

        writeln!(self.output(), "</IconStyle>")?;
        Ok(())
    }

    /// Write out a TimeSpan element.
    fn timespan(&mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> SatFireResult<()> {
        self.output().write_all("<TimeSpan>\n".as_bytes())?;
        writeln!(
            self.output(),
            "<begin>{}</begin>",
            start.format("%Y-%m-%dT%H:%M:%S.000Z")
        )?;
        writeln!(
            self.output(),
            "<end>{}</end>",
            end.format("%Y-%m-%dT%H:%M:%S.000Z")
        )?;
        self.output().write_all("</TimeSpan>\n".as_bytes())?;
        Ok(())
    }

    /// Start a MultiGeometry
    fn start_multi_geometry(&mut self) -> SatFireResult<()> {
        self.output().write_all("<MultiGeometry>\n".as_bytes())?;
        Ok(())
    }

    /// Close out a MultiGeometry
    fn finish_multi_geometry(&mut self) -> SatFireResult<()> {
        self.output().write_all("</MultiGeometry>\n".as_bytes())?;
        Ok(())
    }

    /// Start a Polygon element.
    fn start_polygon(
        &mut self,
        extrude: bool,
        tessellate: bool,
        altitude_mode: Option<&str>,
    ) -> SatFireResult<()> {
        self.output().write_all("<Polygon>\n".as_bytes())?;

        if let Some(altitude_mode) = altitude_mode {
            debug_assert!(
                altitude_mode == "clampToGround"
                    || altitude_mode == "relativeToGround"
                    || altitude_mode == "absolute"
            );

            writeln!(
                self.output(),
                "<altitudeMode>{}</altitudeMode>",
                altitude_mode
            )?;
        }

        if extrude {
            self.output()
                .write_all("<extrude>1</extrude>\n".as_bytes())?;
        }

        if tessellate {
            self.output()
                .write_all("<tessellate>1</tessellate>\n".as_bytes())?;
        }

        Ok(())
    }

    /// Close out a Polygon element.
    fn finish_polygon(&mut self) -> SatFireResult<()> {
        self.output().write_all("</Polygon>\n".as_bytes())?;
        Ok(())
    }

    /// Start the polygon outer ring.
    ///
    /// This should only be used inside a Polygon element.
    ///
    fn polygon_start_outer_ring(&mut self) -> SatFireResult<()> {
        self.output().write_all("<outerBoundaryIs>\n".as_bytes())?;
        Ok(())
    }

    /// End the polygon outer ring.
    ///
    ///  This should only be used inside a Polygon element.
    ///
    fn polygon_finish_outer_ring(&mut self) -> SatFireResult<()> {
        self.output().write_all("</outerBoundaryIs>\n".as_bytes())?;
        Ok(())
    }

    /// Start a LinearRing.
    fn start_linear_ring(&mut self) -> SatFireResult<()> {
        self.output()
            .write_all("<LinearRing>\n<coordinates>\n".as_bytes())?;
        Ok(())
    }

    /// End a LinearRing.
    fn finish_linear_ring(&mut self) -> SatFireResult<()> {
        self.output()
            .write_all("</coordinates>\n</LinearRing>\n".as_bytes())?;
        Ok(())
    }

    /// Add a vertex to the LinearRing
    ///
    /// Must be used inside a linear ring element.
    fn linear_ring_add_vertex(&mut self, lat: f64, lon: f64, z: f64) -> SatFireResult<()> {
        writeln!(self.output(), "{},{},{}", lon, lat, z)?;
        Ok(())
    }

    /// Write out a KML Point element
    fn create_point(&mut self, lat: f64, lon: f64, z: f64) -> SatFireResult<()> {
        writeln!(
            self.output(),
            "<Point>\n<coordinates>{},{},{}</coordinates>\n</Point>",
            lon,
            lat,
            z
        )?;
        Ok(())
    }
}
