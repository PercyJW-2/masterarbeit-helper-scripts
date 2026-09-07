import logging
import subprocess
import sys
import time
from pathlib import Path
from typing import Annotated

import typer
import yaml

logging.basicConfig(level=logging.DEBUG)
logger = logging.getLogger(__name__)

app = typer.Typer()


def start_run(
    args: dict,
    storage_path: Path,
    pico_samplerate_override: None | int = None,
    duration_override: None | int = None,
) -> int:
    logger.debug("building data collection command")
    if pico_samplerate_override is None:
        pico_samplerate_override = args["picoscope_samplerate"]
    command = args["command"]
    if duration_override is None:
        duration_override = args["duration"]
        if duration_override is None:
            sys.exit(-3)
    else:
        command += f" {duration_override}"
    data_collection_command = f"urecs-data-collector -s={storage_path.as_posix()} -d={int(duration_override + 1)}s -c='{command}'"
    if args["jetson"]:
        data_collection_command += f" jetson --address={args['jetson_address']} --data-port=8080 --control-port=8081"
    if args["hailo"]:
        data_collection_command += f" hailo-r-t --address={args['hailo_address']} --data-port=4000 --control-port=4001"
    if args["firmware"]:
        data_collection_command += f" firmware --address={args['firmware_address']}"
    if args["fast_firmware"]:
        data_collection_command += f" fast-firmware --address={args['firmware_address']} --data-port=3000 --channel={args['fast_firmware_channel']} --sample-rate={args['fast_firmware_samplerate']}"
    if args["shelly"]:
        data_collection_command += f" shelly-plug --address={args['shelly_address']}"
    if args["picoscope"]:
        data_collection_command += f" usb-oscilloscope --sample-rate={pico_samplerate_override} --measurement-type={args['picoscope_measurement_type']} --msmt-environment={args['measurement_environment']}"
    if args["tek_scope"]:
        data_collection_command += f" oscilloscope --address{args['tek_scope_address']}"
    logger.info(data_collection_command)
    power_calculation_command = f"power_calculations -m={storage_path.as_posix()} -c -r --estimated-duration={int(duration_override + 2)} --environment={args['measurement_environment']}"
    if args["apply_filter"]:
        power_calculation_command += " -f"
    power_calculation_methods = ""
    power_cut_section_command = ""
    if args["use_complete_measurement"]:
        power_cut_section_command = " --predicted-maximum=0.0001 --predicted-minimum=0"
    if args["fast_firmware"]:
        power_calculation_methods += f" firmware -s={args['fast_firmware_samplerate']}{power_cut_section_command}"
    if args["picoscope"]:
        power_calculation_methods += f" oscilloscope -s={pico_samplerate_override} -m={args['picoscope_measurement_type']}{power_cut_section_command}"
        if args["picoscope_use_measured_voltages"]:
            power_calculation_methods += " -v"
    if args["tek_scope"]:
        power_calculation_methods += (
            f" -s={args['tek_scope_samplerate']}{power_cut_section_command}"
        )
    if args["shelly"]:
        power_calculation_methods += f" shelly{power_cut_section_command}"
    if args["jetson"]:
        power_calculation_methods += f" jetson{power_cut_section_command}"
    if args["hailo"]:
        power_calculation_methods += f" hailo_rt{power_cut_section_command}"

    def execute_run(run_number: int, run_path) -> tuple[bool, bool]:
        if not run_path.exists():
            run_path.mkdir()
        logger.info(f"Starting run number {run_number}")
        p = subprocess.run(data_collection_command, shell=True, check=False)
        try:
            p.check_returncode()
        except subprocess.CalledProcessError:
            logger.error("Recording Failed: retry engaged")
            return False, True
        if args["skip_power_calculation"]:
            logger.info("Moving recorded data into measurement folder")
            output_files = list(storage_path.glob("*.parquet"))
            for file in output_files:
                file.move(run_path)
            return True, False
        logger.info("Starting power calculation")
        iteration_command = (
            power_calculation_command
            + f" --output-path={run_path.as_posix()}"
            + power_calculation_methods
        )
        logger.debug(f"iteration_command: {iteration_command}")
        p = subprocess.run(iteration_command, shell=True, check=False)
        try:
            p.check_returncode()
        except subprocess.CalledProcessError:
            logger.error("Power Calculation Failed: retry engaged")
            return False, True
        logger.debug("Cleaning previous measurements")
        msmts = list(storage_path.glob("*.parquet"))
        for msmt in msmts:
            msmt.unlink()
        return False, False

    invalid_runs = 0
    planned_duration = int(duration_override + 2)

    for run_number in range(args["run_count"]):
        duration_diff = planned_duration + 1
        while duration_diff > planned_duration * 0.1:
            if duration_diff < planned_duration:
                logger.info("Run was invalid, doing run again")
                invalid_runs += 1
            run_path = storage_path / str(run_number)
            skip_power_calculation, run_failed = execute_run(run_number, run_path)
            if run_failed:
                duration_diff = planned_duration * 0.99
                continue
            if skip_power_calculation:
                continue
            logger.debug("Checking run duration")
            duration = 0
            with (run_path / "results.yaml").open() as result_file:
                result = yaml.safe_load(result_file)
                count = 0
                duration_sum = 0
                if result["jetson_results"] is not None:
                    count += 1
                    duration_sum += result["jetson_results"]["duration"]
                if result["shelly_results"] is not None:
                    count += 1
                    duration_sum += result["shelly_results"]["duration"]
                if result["oscilloscope_results"] is not None:
                    count += 1
                    duration_sum += result["oscilloscope_results"]["results"][
                        "duration"
                    ]
                if result["firmware_results"] is not None:
                    count += 1
                    duration_sum += result["firmware_results"]["duration"]
                duration = duration_sum / count
            planned_duration = int(duration_override + 2)
            duration_diff = abs(duration - planned_duration)

    return invalid_runs


@app.command()
def main(
    command: Annotated[
        str, typer.Option(help="command that executes the stresstest to be measured")
    ],
    measurement_path: Annotated[
        str, typer.Option(help="Location where all measurements are stored")
    ],
    run_count: Annotated[
        int, typer.Option(help="Amount of runs that will be executed")
    ],
    duration: Annotated[
        float | None,
        typer.Option(
            help="If the duration is already known it can be provided here. The unit is in seconds"
        ),
    ] = None,
    fast_firmware: Annotated[bool, typer.Option(help="Measure fast-firmware")] = False,
    fast_firmware_samplerate: Annotated[
        int, typer.Option(help="Samplerate of fast_firmware")
    ] = 2000,
    fast_firmware_channel: Annotated[
        int,
        typer.Option(
            help="Select which Channel on the u.RECS is measured, jetson current is channel 2, m.2 is channel 5"
        ),
    ] = 5,
    firmware: Annotated[
        bool,
        typer.Option(
            help="Measure default firmware, data analysis is currently not implemented for this measurement"
        ),
    ] = False,
    firmware_address: Annotated[
        str, typer.Option(help="Address of u.RECS Management Controller")
    ] = "10.42.0.162",
    measurement_environment: Annotated[
        str,
        typer.Option(
            help="u.RECS needs a correction factor that is associated to the measurement environment, options are Static, Jetson and M.2"
        ),
    ] = "M.2",
    picoscope: Annotated[bool, typer.Option(help="Measure picoscope")] = False,
    picoscope_measurement_type: Annotated[
        str,
        typer.Option(
            help="measurement type connected to picoscope, options are UCurrent, CurrentRanger and INA225."
        ),
    ] = "INA225",
    picoscope_samplerate: Annotated[
        int,
        typer.Option(
            help="Samplerate of the scope to be used, choose value between 50 and 5000000"
        ),
    ] = 5_000_000,
    picoscope_use_measured_voltages: Annotated[
        bool,
        typer.Option(
            help="Per default a Voltage estimation is used, as the u.RECS cannot measure the voltage and the setting is used to override the parity between both modes"
        ),
    ] = False,
    tek_scope: Annotated[bool, typer.Option(help="Measure TekScope")] = False,
    tek_scope_address: Annotated[
        str, typer.Option(help="Network Address of the TekScope")
    ] = "10.42.0.48",
    tek_scope_samplerate: Annotated[
        int,
        typer.Option(
            help="Samplerate of the tekscope to be used, choose value configured on hardware"
        ),
    ] = 5_000_000,
    shelly: Annotated[bool, typer.Option(help="Measure shelly plug")] = False,
    shelly_address: Annotated[
        str,
        typer.Option(help="Network Address of the Shelly Plug"),
    ] = "10.42.0.70",
    jetson: Annotated[bool, typer.Option(help="Measure jetson")] = False,
    jetson_address: Annotated[
        str,
        typer.Option(help="Network Address of the Jetson"),
    ] = "10.42.0.44",
    skip_power_calculation: Annotated[
        bool,
        typer.Option(
            help="Skips power calculation and jsut stores the raw and uncalibrated recorded data"
        ),
    ] = False,
    pico_samplerate_sweep: Annotated[
        bool,
        typer.Option(
            help="In the measurement_path folder there are multiple folders that have the format XSps which are used to determine the sample-rate of the measurement"
        ),
    ] = False,
    hailo: Annotated[bool, typer.Option(help="Measure hailo device")] = False,
    hailo_address: Annotated[
        str, typer.Option(help="Network Address of the Hailo Host")
    ] = "10.42.0.44",
    use_complete_measurement: Annotated[
        bool,
        typer.Option(
            help="Use this mode to calculate the energy of the complete measurement without cutting the start or end"
        ),
    ] = False,
    duration_sweep: Annotated[
        bool,
        typer.Option(
            help="In the measurement_path folder there are multiple folders that have the format Xs which are used to determine the measurement duration"
        ),
    ] = False,
    apply_filter: Annotated[
        bool,
        typer.Option(
            help="Applys Lowpass-Filter on u.RECS and oscilloscope data, Frequency=0.25*Samplerate"
        ),
    ] = False,
):
    if fast_firmware and firmware:
        logger.error("Fast-Firmware and Firmware cannot be measured at the same time")
        sys.exit(-1)
    if not (fast_firmware or firmware or picoscope or shelly or jetson):
        logger.error("Choose at least one measurement method")
        sys.exit(-1)

    storage_path = Path(measurement_path)
    if not storage_path.exists():
        logger.error("Choose a folder that exists to store each run")
        sys.exit(-2)

    if pico_samplerate_sweep and duration_sweep:
        logger.error("Only one sweep type is possible at the same time")

    if duration is None and not duration_sweep:
        logger.info("Starting Dry-Run to determine duration")
        start = time.time()
        subprocess.run(command, shell=True, check=True)
        end = time.time()
        duration = end - start

    args = locals()

    invalid_runs = 0
    if pico_samplerate_sweep:
        for directory in [x for x in storage_path.iterdir() if x.is_dir()]:
            folder_name = directory.name
            samplerate = int(folder_name[:-3])
            logger.info(f"Starting Measurements with {samplerate}S/s")
            invalid_runs += start_run(args, directory, samplerate)
    elif duration_sweep:
        duration = 0
        for directory in [x for x in storage_path.iterdir() if x.is_dir()]:
            folder_name = directory.name
            duration = int(folder_name[:-1])
            logger.info(f"Starting Measurements with {duration}s")
            invalid_runs += start_run(args, directory, None, duration)
    else:
        invalid_runs = start_run(args, storage_path)

    (storage_path / f"invalid_runs_{invalid_runs}").touch()


if __name__ == "__main__":
    app()
