SELECT datetime(MAX(mid_point_time), 'unixepoch') FROM clusters WHERE satellite = ? AND sector = ?
