'''Query the databases produced by findfire and connectfire to make graphs.

This module provides functions for querying and graphing the contents of the databases created by
findfire and connectfire.
'''

from collections.abc import Iterable
from datetime import timedelta, datetime, date
import os
import matplotlib.pyplot as plt
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

        assert clusters_path is not None and fires_path is not None

        self._db = sqlite3.connect(fires_path)

        cur = self._db.cursor()
        cur.execute(f"ATTACH DATABASE '{clusters_path}' AS ff")

        cur.close()

        return

    def total_fire_power_time_series(self, fire_id):
        '''Get a time series of fire power and maximum fire temperature.

        Arguments:
            fire_id (int) - an id number from the fire_id column in the fires database.

        Returns:
            A pandas.DataFrame.
        '''

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

        df = pd.read_sql_query(QUERY, self._db, params=(fire_id, ))

        df['st'] = pd.to_datetime(df['st'], unit='s')
        df.rename(columns={
            'st': 'scan start',
            'tp': 'total power',
            'maxt': 'maximum temperature'
        },
                  inplace=True)

        sat = pd.read_sql_query(
            f"SELECT satellite FROM fires WHERE fires.fire_id = ?",
            self._db,
            params=(fire_id, ))
        df.satellite = sat.loc[0]['satellite']

        return df

    def total_fire_power_by_day(self, fire_id, break_hour_z=12):
        '''Groups total fire power by day.

        Arguments:
            fire_id (int) - an id number from the fire_id column in the fires database.
            break_hour_z (int) - the hour of the day in UTC to change days. This defaults to 12 for 
                12Z, which corresponds to 5 AM or 6 AM in the western U.S. This defualt was chosen 
                because it's early morning which is typically when fire activity is at a minimum and
                it corresponds to the beginning of the work day for wildland fire fighters.

        Returns:
            pandas.core.groupby.DataFrameGroupBy object.
        '''
        df = self.total_fire_power_time_series(fire_id)

        def to_second_of_burn_day(val):
            start = (val - timedelta(hours=break_hour_z)).time()
            hours = start.hour
            minutes = start.minute
            seconds = start.second
            return 3600 * hours + 60 * minutes + seconds

        df['second of burn day'] = df['scan start'].map(to_second_of_burn_day)
        df.set_index('scan start', inplace=True)

        def to_date(val):
            return (val - timedelta(hours=break_hour_z)).date

        res = df.groupby(to_date)
        res.satellite = df.satellite

        return res

    def __get_daily_data(self, fire_id, daily_break_hour_z, start, end):
        daily_data = self.total_fire_power_by_day(fire_id, daily_break_hour_z)
        sat = daily_data.satellite

        if start is not None:
            daily_data = tuple(
                (day, data) for day, data in daily_data if day >= start)
        if end is not None:
            daily_data = tuple(
                (day, data) for day, data in daily_data if day <= end)
        if start is None and end is None:
            daily_data = tuple((day, data) for day, data in daily_data)

        return (daily_data, sat)

    def make_daily_fire_power_plot(self,
                        fire_id,
                        daily_break_hour_z=12,
                        start=None,
                        end=None):
        '''Make a plot of the fire power with several rows, where each row is a day.

        Arguments;
            fire_id (int) - an id number from the fire_id column in the fires database. A sequence
                of numbers is also acceptable.
            break_hour_z (int) - the hour of the day in UTC to change days. This defaults to 12 for 
                12Z, which corresponds to 5 AM or 6 AM in the western U.S. This defualt was chosen 
                because it's early morning which is typically when fire activity is at a minimum and
                it corresponds to the beginning of the work day for wildland fire fighters.

        Returns:
            matplotlib.pyplot.Figure
        '''

        if isinstance(fire_id, Iterable):
            fire_id = tuple(fire_id)
        else:
            fire_id = (fire_id, )

        daily_datas, sats = zip(
            *(self.__get_daily_data(fid, daily_break_hour_z, start, end)
              for fid in fire_id))

        max_power = max(val for daily_data in daily_datas
                        for day, data in daily_data
                        for val in data['total power'])

        all_days = list(
            set(day for daily_data in daily_datas for day, data in daily_data))
        all_days.sort()

        tick_positions = [s for s in range(0, 24 * 60 * 60, 3600)]
        base = datetime(2000, 1, 1, daily_break_hour_z, 0, 0)

        def seconds_to_time_str(val):
            tick_time = base + timedelta(seconds=val)
            return tick_time.strftime("%H")

        tick_labels = [seconds_to_time_str(v) for v in tick_positions]

        f, axs = plt.subplots(len(all_days), figsize=(20, len(all_days) * 4))

        prop_cycle = axs[0]._get_lines.prop_cycler

        axs_dict = {day: ax for day, ax in zip(all_days, axs)}

        colors = {}
        for daily_data, sat, fid in zip(daily_datas, sats, fire_id):
            for day, data in daily_data:
                key = str(fid) + str(sat)
                color = colors.get(key)
                ax = axs_dict[day]

                if color is None:
                    color = next(prop_cycle)['color']
                    colors[key] = color

                ax.plot(data['second of burn day'],
                        data['total power'],
                        color=color,
                        label=f"{sat} {day} {fid}")

        for ax in axs:
            ax.set_xticks(tick_positions, tick_labels)
            ax.legend()
            ax.set_ylim(0, max_power)
            ax.set_ylabel("Total Fire Power (MW)")

        axs[-1].set_xlabel("Hour of Day (Z)")

        return f
