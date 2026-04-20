import argparse
import subprocess
import time
import logging
from pathlib import Path
import yaml

logging.basicConfig(level=logging.DEBUG)
logger = logging.getLogger(__name__)

parser = argparse.ArgumentParser(
    prog="MeasruementSuite",
    description="Collects run-duration and then does multiple measurements with an provided command",
    formatter_class=argparse.ArgumentDefaultsHelpFormatter,
)

parser.add_argument(
    "-c",
    "--command",
    help="command that executes the stresstest to be measured",
    required=True,
)
parser.add_argument(
    "-d",
    "--duration",
    required=False,
    help="If the Duration is already known it can be provided here. The Unit is in Seconds",
    type=float,
)
parser.add_argument(
    "-m",
    "--measurement_path",
    help="Location where all measurements are stored",
    required=True,
)
parser.add_argument(
    "-r",
    "--run_count",
    help="Amount of runs that will be executed",
    type=int,
    required=True,
)
parser.add_argument(
    "-f", "--fast_firmware", help="Measure fast-firmware", action="store_true"
)
parser.add_argument(
    "--fast_firmware_samplerate", help="Samplerate of fast_firmware", default=2000
)
parser.add_argument(
    "--fast_firmware_channel",
    help="Select which Channel on the uRECS is measured, default is Jetson Current",
    default=2,
)
parser.add_argument(
    "-F",
    "--firmware",
    help="Measure default firmware, data analysis is currently not implemented for this measurement",
    action="store_true",
)
parser.add_argument(
    "--firmware_address",
    help="Address of u.RECS Management Controller",
    default="10.42.0.162",
)
parser.add_argument("-p", "--picoscope", help="Measure picoscope", action="store_true")
parser.add_argument(
    "--picoscope_samplerate",
    help="Samplerate of the scope to be used, choose value between 50 and 5000000",
    default=5000000,
    type=int,
)
parser.add_argument(
    "--picoscope_measurement_type",
    help="measurement type connected to picoscope, options are UCurrent, CurrentRanger and INA225. Default is INA225",
    default="INA225",
)
parser.add_argument(
    "--picoscope_use_measured_voltages",
    help="Per default a Voltage estimation is used, as the urecs cannot measure the voltage and this setting is used to override the parity between both modes",
    action="store_true",
)
parser.add_argument("-s", "--shelly", help="Measure shelly plug", action="store_true")
parser.add_argument(
    "--shelly_address", help="Network Address of the Shelly Plug", default="10.42.0.70"
)
parser.add_argument("-j", "--jetson", help="Measure jetson", action="store_true")
parser.add_argument(
    "--jetson_address",
    help="Network Address of the Nvidia Jetson",
    default="10.42.0.200",
)
parser.add_argument(
    "--skip_power_calculation",
    help="Skips power calculation and just stores the raw and uncalibrated recorded data",
    action="store_true",
)
parser.add_argument(
    "--pico_samplerate_sweep",
    help="In the measurement_path folder there are multiple folders that have the format XSps which are used to determine the sample-rate of the measurement",
    action="store_true",
)
parser.add_argument(
    "--use_complete_measurement",
    help="Use this mode to calculate the energy of the complete measurement without cutting the start or end",
    action="store_true",
)


def start_run(
    args: argparse.Namespace,
    storage_path: Path,
    pico_samplerate_override: None | int = None,
):
    logger.debug("building data collection command")
    if pico_samplerate_override is None:
        pico_samplerate_override = args.picoscope_samplerate
    data_collection_command = f"urecs-data-collector -s={storage_path.as_posix()} -d={int(args.duration + 1)}s -c='{args.command}'"
    if args.jetson:
        data_collection_command += f" jetson --address={args.jetson_address} --data-port=8000 --control-port=8081"
    if args.firmware:
        data_collection_command += f" firmware --address={args.firmware_address}"
    if args.fast_firmware:
        data_collection_command += f" fast-firmware --address={args.firmware_address} --data-port=3000 --channel={args.fast_firmware_channel}, --sample-rate={args.fast_firmware_samplerate}"
    if args.shelly:
        data_collection_command += f" shelly-plug --address={args.shelly_address}"
    if args.picoscope:
        data_collection_command += f" usb-oscilloscope --sample-rate={pico_samplerate_override} --measurement-type={args.picoscope_measurement_type}"
    power_calculation_command = f"power_calculations -m={storage_path.as_posix()} -c -r --estimated-duration={int(args.duration + 2)}"
    power_calculation_methods = ""
    power_cut_section_command = ""
    if args.use_complete_measurement:
        power_cut_section_command = " --predicted-maximum=0.0001 --predicted-minimum=0"
    if args.fast_firmware:
        power_calculation_methods += (
            f" firmware -s={args.fast_firmware_samplerate}{power_cut_section_command}"
        )
    if args.picoscope:
        power_calculation_methods += f" oscilloscope -s={pico_samplerate_override} -m={args.picoscope_measurement_type}{power_cut_section_command}"
        if args.picoscope_use_measured_voltages:
            power_calculation_methods += " -v"
    if args.shelly:
        power_calculation_methods += f" shelly{power_cut_section_command}"
    if args.jetson:
        power_calculation_methods += f" jetson{power_cut_section_command}"

    def execute_run(run_number: int, run_path) -> bool:
        if not run_path.exists():
            run_path.mkdir()
        logger.info(f"Starting run number {run_number}")
        subprocess.run(data_collection_command, shell=True)
        if args.skip_power_calculation:
            logger.info("Moving recorded data into measurement folder")
            output_files = list(storage_path.glob("*.parquet"))
            for file in output_files:
                file.move(run_path)
            return True
        logger.info("Starting power calculation")
        iteration_command = (
            power_calculation_command
            + f" --output-path={run_path.as_posix()}"
            + power_calculation_methods
        )
        logger.debug(f"iteration_command: {iteration_command}")
        subprocess.run(iteration_command, shell=True)
        logger.debug("Cleaning previous measurements")
        msmts = list(storage_path.glob("*.parquet"))
        for msmt in msmts:
            msmt.unlink()
        return False

    for run_number in range(args.run_count):
        run_path = storage_path / str(run_number)
        skip_power_calculation = execute_run(run_number, run_path)
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
                duration_sum += result["oscilloscope_results"]["results"]["duration"]
            if result["firmware_results"] is not None:
                count += 1
                duration_sum += result["firmware_results"]["duration"]
            duration = duration_sum / count
        planned_duration = int(args.duration + 1)
        duration_diff = abs(duration - planned_duration)
        if duration_diff > planned_duration * 0.1:
            logger.info(
                "run duration is too long -> scrapping last run and starting run again"
            )
            _ = execute_run(run_number, run_path)


if __name__ == "__main__":
    args = parser.parse_args()

    if args.fast_firmware and args.firmware:
        logger.error("Fast-Firmware and Firmware cannot be measured at the same time")
        exit(-1)
    if not (
        args.fast_firmware
        or args.firmware
        or args.picoscope
        or args.shelly
        or args.jetson
    ):
        logger.error("Choose at least one measurement method")
        exit(-1)

    storage_path = Path(args.measurement_path)
    if not storage_path.exists():
        logger.error("Choose a folder that exists to store each run")
        exit(-2)

    if args.duration is None:
        logger.info("Starting Dry-Run to determine duration")
        start = time.time()
        subprocess.run(args.command, shell=True)
        end = time.time()
        args.duration = end - start

    if args.pico_samplerate_sweep:
        for directory in [x for x in storage_path.iterdir() if x.is_dir()]:
            folder_name = directory.name
            samplerate = int(folder_name[:-3])
            logger.info(f"Starting Measurements with {samplerate}S/s")
            start_run(args, directory, samplerate)
    else:
        start_run(args, storage_path)
