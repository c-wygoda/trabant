#!/usr/bin/env -S uv run --script
# /// script
# dependencies = [
#   "skyfield",
# ]
# ///
"""
Generate satellite pass prediction benchmark data using Python Skyfield.

This script creates reference data for validating the Rust/WASM implementation
of the Trabant satellite pass prediction system.
"""

from datetime import UTC, datetime, timedelta
from pathlib import Path
import json

from skyfield.api import load, wgs84
from skyfield.sgp4lib import EarthSatellite
from skyfield.toposlib import itrs

# Configuration
BERLIN_LAT = 52.5200  # degrees North
BERLIN_LON = 13.4050  # degrees East
BERLIN_ALT = 34  # meters above sea level
MIN_ELEVATION = 15.0  # degrees
DAYS = 28

BASE_DIR = Path(__file__).parent / ".."
FIXTURE_DIR = BASE_DIR / "tests/fixtures"


def load_omm_data():
    """Load HOTSAT-1 OMM data from fixture file."""
    omm_file = FIXTURE_DIR / "hotsat1-omm.json"
    return json.loads(omm_file.read_text())


def find_satellite_passes(omm):
    """Find all satellite passes over Berlin for the specified time period."""
    # Load time scale
    ts = load.timescale()

    # Create satellite object
    satellite = EarthSatellite.from_omm(ts, omm)

    # Set up observer location (Berlin)
    berlin = wgs84.latlon(BERLIN_LAT, BERLIN_LON, BERLIN_ALT)

    epoch_time = datetime.fromisoformat(omm["EPOCH"]).replace(tzinfo=UTC)
    start_time = ts.from_datetime(epoch_time)
    end_time = ts.from_datetime(epoch_time + timedelta(days=14))

    print(
        f"Searching for passes from {epoch_time} to {epoch_time + timedelta(days=DAYS)}"
    )

    # Find passes
    t, events = satellite.find_events(
        berlin, start_time, end_time, altitude_degrees=MIN_ELEVATION
    )

    passes = []
    current_pass = {}

    for ti, event in zip(t, events):
        event_time = ti.utc_datetime()

        if event == 0:  # AOS (Acquisition of Signal)
            current_pass = {
                "start_time": event_time.isoformat(),
                "start_time_unix": event_time.timestamp(),
                "start_time_jd": ti.tt,
            }
        elif event == 1:  # Maximum elevation
            # Calculate position at max elevation
            difference = satellite - berlin
            topocentric = difference.at(ti)
            alt, az, distance = topocentric.altaz()

            current_pass.update(
                {
                    "max_elevation_time": event_time.isoformat(),
                    "max_elevation_time_unix": event_time.timestamp(),
                    "max_elevation_time_jd": ti.tt,
                    "max_elevation": alt.degrees,
                    "max_elevation_azimuth": az.degrees,
                    "max_elevation_range": distance.km,
                }
            )
        elif event == 2:  # LOS (Loss of Signal)
            current_pass.update(
                {
                    "end_time": event_time.isoformat(),
                    "end_time_unix": event_time.timestamp(),
                    "end_time_jd": ti.tt,
                }
            )

            if "start_time_unix" in current_pass and "end_time_unix" in current_pass:
                passes.append(current_pass)
                current_pass = {}

    return [start_time, end_time, passes]

def find_positions(omm):
    # Load time scale
    ts = load.timescale()

    # Create satellite object
    satellite = EarthSatellite.from_omm(ts, omm)

    epoch_time = datetime.fromisoformat(omm["EPOCH"]).replace(tzinfo=UTC)
    positions = []
    for i in range(1000):
        t = ts.from_datetime(epoch_time + timedelta(seconds=i))
        pos_teme, velocity_teme, _ = satellite._position_and_velocity_TEME_km(t)
        gcrs = satellite.at(t)
        pos_itrs, velocity_itrs = gcrs.frame_xyz_and_velocity(itrs)
        earth = wgs84.subpoint_of(gcrs)
        positions.append({
            "t_utc": t.utc_datetime().isoformat(),
            "t_jd": t.tt,
            "state_teme": {
                "pos": list(pos_teme),
                "velocity": list(velocity_teme)
            },
            "state_gcrs": {
                "pos": list(pos_itrs.km),
                "velocity": list(velocity_itrs.m_per_s),
            },
            "wgs84_location": {
                "latitude": earth.latitude.degrees,
                "longitude": earth.longitude.degrees,
            }
        })
    return positions


def main():
    """Generate benchmark fixture data."""
    print("Generating satellite pass benchmark data...")
    omm = load_omm_data()

    # Find all passes
    start_time, end_time, passes = find_satellite_passes(omm)

    print(f"Found {len(passes)} passes above {MIN_ELEVATION}° elevation")

    # Create benchmark data structure
    benchmark_data = {
        "omm": omm,
        "parameters": {
            "location": {
                "latitude": BERLIN_LAT,
                "longitude": BERLIN_LON,
                "altitude": BERLIN_ALT,
            },
            "min_elevation": MIN_ELEVATION,
            "start_time": start_time.utc_iso(),
            "end_time": end_time.utc_iso(),
        },
        "passes": passes,
    }

    # Save to file
    output_file = BASE_DIR / "tests/fixtures/hotsat1-berlin-passes.json"
    output_file.write_text(json.dumps(benchmark_data, indent=2))

    print(f"Benchmark data saved to {output_file}")

    # Print summary
    if passes:
        print(f"\nPass summary:")
        for i, pass_data in enumerate(passes):
            start = datetime.fromisoformat(pass_data["start_time"])
            max_el = pass_data["max_elevation"]
            duration = pass_data["end_time_unix"] - pass_data["start_time_unix"]
            print(
                f"  Pass {i + 1}: {start.strftime('%Y-%m-%d %H:%M:%S')} UTC, "
                f"Max elevation: {max_el:.1f}°, Duration: {duration / 60:.1f}min"
            )

    positions = find_positions(omm)
    output_file = BASE_DIR / "tests/fixtures/hotsat1-positions.json"
    output_file.write_text(json.dumps(positions, indent=2))


if __name__ == "__main__":
    main()
