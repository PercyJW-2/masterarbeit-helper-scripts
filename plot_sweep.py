from typing import Dict
import matplotlib.pyplot as plt
from matplotlib.axes import Axes
import yaml
import argparse
from pathlib import Path
import numpy as np

parser = argparse.ArgumentParser("Plot samplerate sweep done with measurement suite")

parser.add_argument("-p", "--path", help="Root folder of msmt sweep", required=True)
parser.add_argument(
    "--constant_cut",
    help="Do calculation manually while keeping the measurement duration constant",
    action="store_true",
)
parser.add_argument(
    "--virtual_samplerates",
    help="This estimates each samplerate by loading the most detailed one and removing samples until each samplerate is reached",
    action="store_true",
)


def calculate_energy(data: np.ndarray, samplerate):
    return np.sum(((data[:-1] + data[1:]) / 2) * (1 / samplerate))


def load_all_data(
    path: Path, constant_cut: bool
) -> Dict[int, tuple[list[float], list[float]]]:
    results: Dict[int, tuple[list[float], list[float]]] = dict()
    for folder in [x for x in path.iterdir() if x.is_dir()]:
        print("Currently in Folder:", folder.as_posix())
        samplerate = int(folder.name[:-3])
        energy = []
        duration = []
        for run in [x for x in folder.iterdir() if x.is_dir()]:
            if constant_cut:
                data = np.load((run / "oscilloscope.npy").as_posix())
                samples = int(60 / (1 / samplerate))
                print(
                    f"samplerate {samplerate}\tsamples {samples}\tduration {samples / samplerate}"
                )
                data = data[: int(60 / (1 / samplerate))]
                energy.append(calculate_energy(data, samplerate))
                duration.append(samples / samplerate)
                del data
            else:
                result_path = run / "results.yaml"
                if not result_path.exists():
                    continue
                with result_path.open() as result_file:
                    result = yaml.safe_load(result_file)["oscilloscope_results"][
                        "results"
                    ]
                    energy.append(result["energy"])
                    duration.append(result["duration"])
        results[samplerate] = (energy, duration)
    return results


def estimate_data(
    path: Path, constant_cut: bool
) -> Dict[int, tuple[list[float], list[float]]]:
    results = dict()
    samplerates = [int(x.name[:-3]) for x in path.iterdir() if x.is_dir()]
    samplerates.sort()

    max_samplerate_folder = path / f"{samplerates[-1]}Sps"
    for run in [x for x in max_samplerate_folder.iterdir() if x.is_dir()]:
        data = np.load((run / "oscilloscope.npy").as_posix())
        duration = 0.0
        if constant_cut:
            data = data[: int(60 / (1 / samplerates[-1]))]
            duration = 60
        else:
            with (run / "results.yaml").open() as result_file:
                result = yaml.safe_load(result_file)
                start_stop_idx = result["oscilloscope_results"]["results"][
                    "start_stop_idx"
                ]
                data = data[start_stop_idx[0] : start_stop_idx[1]]
                duration = result["oscilloscope_results"]["results"]["duration"]
        # loaded data of run
        for samplerate in samplerates:
            skip_amount = round(samplerates[-1] / samplerate)
            actual_samplerate = round(samplerates[-1] / skip_amount)
            print(
                f"skip_amount: {skip_amount} actual_samplerate: {actual_samplerate} samplerate: {samplerate} max_samplerate: {samplerates[-1]}"
            )
            if actual_samplerate not in results:
                results[actual_samplerate] = ([], [])
            results[actual_samplerate][0].append(
                calculate_energy(data[::skip_amount], actual_samplerate)
            )
            results[actual_samplerate][1].append(duration)

    return results


def plot_as_boxplot(data: list[list[float]], ax: Axes):
    ax.boxplot(data, showfliers=False)


def plot_as_scatterplot(data: list[list[float]], ax: Axes):
    for idx, samplerate_data in enumerate(data):
        ax.scatter([idx + 1] * len(samplerate_data), samplerate_data)


def plot_data(data: list[list[float]], ax: Axes):
    if len(data[0]) > 10:
        plot_as_boxplot(data, ax)
    else:
        plot_as_scatterplot(data, ax)


if __name__ == "__main__":
    args = parser.parse_args()
    path = Path(args.path)

    results = None
    if args.virtual_samplerates:
        results = estimate_data(path, args.constant_cut)
    else:
        results = load_all_data(path, args.constant_cut)
    samplerates = list(results.keys())
    samplerates.sort()

    energies = []
    durations = []
    normalized_energies = []
    for samplerate in samplerates:
        energy, duration = results[samplerate]
        normalized_energy = [e / d for e, d in zip(energy, duration)]
        energies.append(energy)
        durations.append(duration)
        normalized_energies.append(normalized_energy)

    fig, axs = plt.subplots(1, 3)
    plot_data(energies, axs[0])
    axs[0].set_ylabel("Energy (J)")
    plot_data(durations, axs[1])
    axs[1].set_ylabel("Duration (s)")
    plot_data(normalized_energies, axs[2])
    axs[2].set_ylabel("Watt (J/s)")
    for ax in axs:
        ax.tick_params("x", rotation=90)
        ax.xaxis.grid(True)
        ax.yaxis.grid(True)
        ax.set_xticks(np.arange(1, len(samplerates) + 1), labels=samplerates)
        ax.set_xlabel("Samplerate")
    plt.show()
