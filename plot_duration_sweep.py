from typing import Dict
from enum import Enum
from matplotlib.axes import Axes
from matplotlib.figure import Figure
import matplotlib.pyplot as plt
import yaml
import argparse
from pathlib import Path
import numpy as np

type Duration = float
type Energy = float


class MsmtType(Enum):
    PICO = 0
    URECS = 1
    JETSON = 2
    SHELLY = 3

    def load_power_data(self, path: Path) -> np.ndarray:
        match self:
            case MsmtType.PICO:
                return np.load((path / "oscilloscope.npy").as_posix())
            case MsmtType.URECS:
                return np.load((path / "firmware_power.npy").as_posix())
            case MsmtType.JETSON:
                return np.load((path / "jetson.npy").as_posix())
            case MsmtType.SHELLY:
                return np.load((path / "shelly.npy").as_posix())


parser = argparse.ArgumentParser("Plot duration sweep done with measurement suite")

parser.add_argument("-p", "--path", help="Root folder of msmt sweep", required=True)


def load_all_data(
    path: Path,
) -> Dict[int, list[Dict[MsmtType, tuple[Duration, Energy]]]]:
    data = dict()
    for folder in [x for x in path.iterdir() if x.is_dir()]:
        duration = int(folder.name[:-1])
        msmts = []
        for run in [x for x in folder.iterdir() if x.is_dir()]:
            result_path = run / "results.yaml"
            if not result_path.exists():
                continue
            run_results = dict()
            with result_path.open() as result_file:
                result = yaml.safe_load(result_file)
                run_results[MsmtType.PICO] = (
                    result["oscilloscope_results"]["results"]["duration"],
                    result["oscilloscope_results"]["results"]["energy"],
                )
                run_results[MsmtType.URECS] = (
                    result["firmware_results"]["duration"],
                    result["firmware_results"]["energy"],
                )
                run_results[MsmtType.JETSON] = (
                    result["jetson_results"]["duration"],
                    result["jetson_results"]["energy"],
                )
                run_results[MsmtType.SHELLY] = (
                    result["shelly_results"]["duration"],
                    result["shelly_results"]["energy"],
                )
            msmts.append(run_results)
        data[duration] = msmts
    return data


def convert_run_data(
    run_data: list[Dict[MsmtType, tuple[Duration, Energy]]],
) -> tuple[
    tuple[list[Duration], list[Duration], list[Duration], list[Duration]],
    tuple[list[Energy], list[Energy], list[Energy], list[Energy]],
]:
    pico_durations = []
    urecs_durations = []
    jetson_durations = []
    shelly_durations = []
    pico_energies = []
    urecs_energies = []
    jetson_energies = []
    shelly_energies = []
    for run in run_data:
        pico_durations.append(run[MsmtType.PICO][0])
        pico_energies.append(run[MsmtType.PICO][1])
        urecs_durations.append(run[MsmtType.URECS][0])
        urecs_energies.append(run[MsmtType.URECS][1])
        jetson_durations.append(run[MsmtType.JETSON][0])
        jetson_energies.append(run[MsmtType.JETSON][1])
        shelly_durations.append(run[MsmtType.SHELLY][0])
        shelly_energies.append(run[MsmtType.SHELLY][1])
    return (
        (pico_durations, urecs_durations, jetson_durations, shelly_durations),
        (
            pico_energies,
            urecs_energies,
            jetson_energies,
            shelly_energies,
        ),
    )


if __name__ == "__main__":
    args = parser.parse_args()
    path = Path(args.path)

    data = load_all_data(path)

    durations = list(data.keys())
    durations.sort()

    ret: tuple[Figure, np.ndarray] = plt.subplots(2, len(durations) // 2)
    fig, axs = ret
    axs_l: list[Axes] = list(axs.ravel())

    fig.set_size_inches((20, 10))

    for ax, duration in zip(axs_l, durations):
        duration_data = data[duration]
        transposed_data = convert_run_data(duration_data)

        duration_data = np.array(transposed_data[0])
        energy_data = np.array(transposed_data[1])
        joule_per_second_data = energy_data / duration_data

        median_jps = np.median(joule_per_second_data, axis=1)
        print(median_jps)

        ax.boxplot(joule_per_second_data.T, showfliers=False)
        ax.set_title(f"{duration}s")
        ax.set_ylabel("Energy (J)")
        ax.set_xticks([1, 2, 3, 4], labels=["Picoscope", "u.RECS", "Jetson", "Shelly"])
        ax.tick_params("x", rotation=90)
    fig.tight_layout()
    plt.show()
