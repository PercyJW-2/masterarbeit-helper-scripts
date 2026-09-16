from enum import Enum
from matplotlib.axes import Axes
from matplotlib.figure import Figure, FigureBase
import matplotlib.pyplot as plt
import yaml
import argparse
from pathlib import Path
import numpy as np

type Duration = float
type Energy = float


class MsmtType(Enum):
    PICO = "Pico"
    URECS = "u.RECS"
    JETSON = "Jetson"
    SHELLY = "Shelly"
    HAILO = "Hailo"

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
            case MsmtType.HAILO:
                return np.load((path / "hailo_rt.npy").as_posix())


parser = argparse.ArgumentParser("Plot duration sweep done with measurement suite")

parser.add_argument("-p", "--path", help="Root folder of msmt sweep", required=True)
parser.add_argument("-s", "--skip_plot", action="store_true")


def load_all_data(
    path: Path,
) -> dict[int, list[dict[MsmtType, tuple[Duration, Energy]]]]:
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
                if result["jetson_results"] is not None:
                    run_results[MsmtType.JETSON] = (
                        result["jetson_results"]["duration"],
                        result["jetson_results"]["energy"],
                    )
                run_results[MsmtType.SHELLY] = (
                    result["shelly_results"]["duration"],
                    result["shelly_results"]["energy"],
                )
                if result["hailo_rt_results"] is not None:
                    run_results[MsmtType.HAILO] = (
                        result["hailo_rt_results"]["duration"],
                        result["hailo_rt_results"]["energy"],
                    )
            msmts.append(run_results)
        data[duration] = msmts
    return data


def convert_run_data(
    run_data: list[dict[MsmtType, tuple[Duration, Energy]]],
) -> tuple[dict[MsmtType, np.ndarray], dict[MsmtType, np.ndarray]]:
    durations: dict[MsmtType, np.ndarray] = {}
    energies: dict[MsmtType, np.ndarray] = {}
    for run in run_data:
        for key, value in run.items():
            if key not in durations:
                durations[key] = np.array([value[0]])
                energies[key] = np.array([value[1]])
            else:
                durations[key] = np.append(durations[key], value[0])
                energies[key] = np.append(energies[key], value[1])
    return (durations, energies)


if __name__ == "__main__":
    args = parser.parse_args()
    path = Path(args.path)

    data = load_all_data(path)

    durations = list(data.keys())
    durations.sort()

    returned: tuple[Figure, np.ndarray] = plt.subplots(
        2, len(durations) // 2, sharey=True
    )
    fig, axs = returned
    axs_l: list[Axes] = list(axs.ravel())

    fig.set_size_inches((10, 5))

    y_label_font_size = 12

    median_energy_values = []
    median_joule_per_second_values = []

    for ax, duration in zip(axs_l, durations):
        duration_data = data[duration]
        transposed_data = convert_run_data(duration_data)

        duration_data = transposed_data[0]
        energy_data = transposed_data[1]
        joule_per_second_data: dict[MsmtType, np.ndarray] = {}
        for key, duration_key in duration_data.items():
            joule_per_second_data[key] = np.array(duration_key) / np.array(
                energy_data[key]
            )
        median_jps = np.median(list(joule_per_second_data.values()), axis=1)
        median_joule_per_second_values.append(median_jps)

        median_energy = np.median(list(energy_data.values()), axis=1)
        median_energy_values.append(median_energy)
        print(median_energy)

        ax.boxplot(list(joule_per_second_data.values()), showfliers=False)
        ax.set_title(f"{duration}s")
        ax.set_xticks(
            [1, 2, 3, 4], labels=[key.value for key in joule_per_second_data.keys()]
        )
        ax.tick_params("x", rotation=90)
        ax.yaxis.grid(True)
    axs_l[0].set_ylabel("Energy per Second (J/s)", fontsize=y_label_font_size)
    axs_l[len(durations) // 2].set_ylabel(
        "Energy per Second (J/s)", fontsize=y_label_font_size
    )
    fig.tight_layout()
    plt.savefig("duration_sweep_boxplot.pdf")

    plt.figure()

    plt.plot(
        [5, 10, 20, 50, 100, 200, 300, 400, 500, 600],
        [x[0] for x in median_joule_per_second_values],
    )
    plt.plot(
        [5, 10, 20, 50, 100, 200, 300, 400, 500, 600],
        [x[1] for x in median_joule_per_second_values],
    )
    plt.plot(
        [5, 10, 20, 50, 100, 200, 300, 400, 500, 600],
        [x[2] for x in median_joule_per_second_values],
    )
    plt.plot(
        [5, 10, 20, 50, 100, 200, 300, 400, 500, 600],
        [x[3] for x in median_joule_per_second_values],
    )
    if not args.skip_plot:
        plt.show()

    ret: tuple[Figure, np.ndarray] = plt.subplots(1, len(durations), sharey=True)
    fig, axs = ret
    axs_l: list[Axes] = list(axs.ravel())

    fig.set_size_inches((10, 2.5))

    urecs_diffs = []
    for ax, duration, median_energy in zip(axs_l, durations, median_energy_values):
        perc_diffs = (median_energy[1:] - median_energy[0]) / median_energy[0]
        perc_diffs *= 100
        urecs_diffs.append(perc_diffs[0])
        print(duration, ": ", perc_diffs)

        ax.bar(np.arange(1, 4), perc_diffs, fill=False, hatch="//")
        ax.set_title(f"{duration}s")
        names = set(duration_data.keys())
        names.remove(MsmtType.PICO)
        ax.set_xticks([1, 2, 3], labels=[name.value for name in names])
        ax.tick_params("x", rotation=90)
        ax.yaxis.grid(True)
    axs_l[0].set_ylabel("Percent (%)", fontsize=y_label_font_size)
    print(np.median(urecs_diffs))
    fig.tight_layout()
    plt.savefig("duration_sweep_deviations.pdf")
    if not args.skip_plot:
        plt.show()

    median_energy_values = np.array(median_joule_per_second_values)
    print(median_energy_values)
    diff_between_5s_and_600s = (
        median_energy_values[:-1, 0] - median_energy_values[-1][0]
    ) / median_energy_values[-1][0]

    print("Perc_diffs Picoscope 5s towards 600s: ", diff_between_5s_and_600s[0] * 100)
    print("Perc_diffs Picoscope 100s towards 600s: ", diff_between_5s_and_600s[4] * 100)
    print(durations)
    print(
        "First Msmt below 3%:",
        durations[np.argmax(diff_between_5s_and_600s > -0.03)],
        diff_between_5s_and_600s[np.argmax(diff_between_5s_and_600s > -0.03)] * 100,
    )
