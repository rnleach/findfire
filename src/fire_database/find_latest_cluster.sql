SELECT datetime(MAX(start_time), 'unixepoch') FROM clusters WHERE satellite = ? AND sector = ?
