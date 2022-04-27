'''Query the databases produced by findfire and connectfire.

This module provides functions for querying and graphing the contents of the databases created by
findfire and connectfire.
'''

import os
import pandas as pd
import sqlite3

class SatfireDatabases:
    '''A read only handle to the clusters and fires databases.'''

    def __init__(self, clusters=None, fires=None):
        if clusters is None:
            clusters_path = os.getenv('CLUSTER_DB')
        else:
            clusters_path = clusters

        if fires is None:
            fires_path = os.getenv('FIRES_DB')
        else:
            fires_path = fires

        self._db = sqlite3.connect(fires_path)

        cur = self._db.cursor()
        cur.execute(f"ATTACH DATABASE '{clusters_path}' AS ff")

        cur.close()

        return

    def total_fire_power_time_series(self, fire_id):
        '''Get a time series of datetime objects and fire powers.'''

        QUERY = """
            SELECT
                ff.clusters.start_time as st, 
                SUM(ff.clusters.power) as tp, 
                MAX(ff.clusters.max_temperature) as maxt
            FROM associations  JOIN ff.clusters ON ff.clusters.cluster_id = associations.cluster_id
            WHERE associations.fire_id in (
                WITH RECURSIVE
                    find_mergers(x) AS (
                        VALUES(?)
                        UNION ALL
                        SELECT fire_id FROM fires, find_mergers WHERE merged_into = find_mergers.x
                    )
                SELECT fire_id FROM fires
                WHERE fire_id IN find_mergers
                ORDER BY fire_id ASC
            )
            GROUP BY  st
            ORDER BY st ASC
        """

        df = pd.read_sql_query(QUERY, self._db, params=(fire_id,))

        df['st'] = pd.to_datetime(df['st'], unit='s')
        df.rename(columns={'st':'scan start', 'tp':'total power', 'maxt':'maximum temperature'},
                inplace=True)

        return df


