from typing import Dict
import matplotlib.pyplot as plt
from matplotlib.axes import Axes
import yaml
import argparse
from pathlib import Path
import numpy as np
import scipy.signal

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
parser.add_argument(
    "--simulate_slow_samplerates",
    help="This estimates samplerates below 50S/s, as picoscope cannot measure lower than that",
    action="store_true",
)
parser.add_argument(
    "--fit_max_energy",
    help="Recalculates max energy by searching for it",
    action="store_true",
)


def calculate_energy(data: np.ndarray, samplerate):
    return np.sum(((data[:-1] + data[1:]) / 2) * (1 / samplerate))


def load_all_data(
    path: Path, constant_cut: bool, search_max_energy: bool
) -> Dict[int, tuple[list[float], list[float], list[float]]]:
    results: Dict[int, tuple[list[float], list[float], list[float]]] = dict()
    for folder in [x for x in path.iterdir() if x.is_dir()]:
        print("Currently in Folder:", folder.as_posix())
        samplerate = int(folder.name[:-3])
        energy = []
        duration = []
        sample_count = []
        for run in [x for x in folder.iterdir() if x.is_dir()]:
            if constant_cut and not search_max_energy:
                data = np.load((run / "oscilloscope.npy").as_posix())
                samples = int(60 / (1 / samplerate))
                print(
                    f"samplerate {samplerate}\tsamples {samples}\tduration {samples / samplerate}"
                )
                sample_count.append(int(60 / (1 / samplerate)))
                data = data[: int(60 / (1 / samplerate))]
                energy.append(calculate_energy(data, samplerate))
                duration.append(samples / samplerate)
                del data
            elif search_max_energy:
                data = np.load((run / "oscilloscope.npy").as_posix())
                samples = 0
                previous_energy = 0
                if constant_cut:
                    samples = int(60 / (1 / samplerate))
                    previous_energy = calculate_energy(data[:samples], samplerate)
                else:
                    with (run / "results.yaml").open() as result_file:
                        result = yaml.safe_load(result_file)["oscilloscope_results"][
                            "results"
                        ]
                        previous_energy = result["energy"]
                        samples = (
                            result["start_stop_idx"][1] - result["start_stop_idx"][0]
                        )
                current_max = 0
                for i in np.arange(data.shape[0] - samples):
                    nrg = calculate_energy(data[i : i + samples], samplerate)
                    if nrg > current_max:
                        current_max = nrg
                print("Energy Diff: ", abs(current_max - previous_energy))
                energy.append(current_max)
                duration.append(samples / samplerate)
                sample_count.append(samples)
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
                    sample_count.append(
                        result["start_stop_idx"][1] - result["start_stop_idx"][0]
                    )
        if len(energy) != 0:
            results[samplerate] = (energy, duration, sample_count)
    return results


def estimate_data(
    path: Path,
    constant_cut: bool,
    samplerates: list[int],
    original_samplerate: int,
    results: Dict[int, tuple[list[float], list[float], list[float]]],
) -> None:
    max_samplerate_folder = path / f"{original_samplerate}Sps"
    for run in [x for x in max_samplerate_folder.iterdir() if x.is_dir()]:
        data = np.load((run / "oscilloscope.npy").as_posix())
        if constant_cut:
            data = data[: int(60 / (1 / original_samplerate))]
        else:
            with (run / "results.yaml").open() as result_file:
                result = yaml.safe_load(result_file)
                start_stop_idx = result["oscilloscope_results"]["results"][
                    "start_stop_idx"
                ]
                data = data[start_stop_idx[0] : start_stop_idx[1]]
        # loaded data of run
        simulate_samplerates(samplerates, original_samplerate, data, results)


def simulate_samplerates(
    samplerates: list[int],
    original_samplerate: int,
    data: np.ndarray,
    result_dict: Dict[int, tuple[list[float], list[float], list[float]]],
) -> None:
    for samplerate in samplerates:
        skip_amount = round(original_samplerate / samplerate)
        actual_samplerate = round(original_samplerate / skip_amount)
        print(
            f"skip_amount: {skip_amount} actual_samplerate: {actual_samplerate} samplerate: {samplerate} max_samplerate: {original_samplerate}"
        )
        sos = scipy.signal.butter(
            1, actual_samplerate / 2, "lp", fs=original_samplerate, output="sos"
        )
        filtered_data = scipy.signal.sosfilt(sos, data)
        if actual_samplerate not in result_dict:
            result_dict[actual_samplerate] = ([], [], [])
        result_dict[actual_samplerate][0].append(
            calculate_energy(filtered_data[::skip_amount], actual_samplerate)
        )
        result_dict[actual_samplerate][1].append(
            filtered_data[::skip_amount].shape[0] / actual_samplerate
        )
        result_dict[actual_samplerate][2].append(filtered_data[::skip_amount].shape[0])


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

    results = dict()
    samplerates = [int(x.name[:-3]) for x in path.iterdir() if x.is_dir()]
    samplerates.sort()

    if args.virtual_samplerates:
        estimate_data(path, args.constant_cut, samplerates, samplerates[-1], results)
    else:
        results = load_all_data(path, args.constant_cut, args.search_max_energy)
    if args.simulate_slow_samplerates:
        estimate_data(path, args.constant_cut, [1, 5, 10, 25], 50, results)

    samplerates = list(results.keys())
    samplerates.sort()

    energies = []
    durations = []
    normalized_energies = []
    energy_per_sample = []
    samplecounts = []
    for samplerate in samplerates:
        energy, duration, samplecount = results[samplerate]
        normalized_energy = [e / d for e, d in zip(energy, duration)]
        energies.append(energy)
        durations.append(duration)
        normalized_energies.append(normalized_energy)
        energy_per_sample.append([e / sc for e, sc in zip(energy, samplecount)])
        samplecounts.append(samplecount)

    fig, axs = plt.subplots(1, 3)
    fig.set_size_inches((20, 10))
    axs: list[Axes] = axs
    # ax0 = axs[0].twinx()
    # ax0.set_yscale("log")
    # ax0.bar(
    #    range(1, len(samplecounts) + 1),
    #    np.median(energy_per_sample, axis=1),
    #    fill=True,
    # )
    plot_data(energies, axs[0])
    axs[0].set_ylabel("Energy (J)")
    axs[0].set_title("Measurement Energies")
    # axs[0].set_zorder(1)
    # axs[0].set_frame_on(False)

    # plot_data(durations, axs[1])
    # axs[1].set_ylabel("Duration (s)")
    # plot_data(normalized_energies, axs[1])
    # axs[1].set_ylabel("Watt (J/s)")
    # axs[1].set_title("Average Power")
    energies = np.array(energies)
    """axs[1].bar(
        range(1, energies.shape[0] + 1),
        (
            np.std(
                energies - np.median(energies, axis=1, keepdims=True), axis=1, ddof=1
            )
            / np.mean(energies, axis=1)
        )
        * 100,
        fill=False,
    )
    axs[1].set_ylabel("Percent (%)")
    axs[1].set_title("Measurement Std in Percent")"""
    q1 = np.percentile(energies, 25, axis=1)
    q3 = np.percentile(energies, 75, axis=1)
    quantile_deviation = (q3 - q1) / 2
    axs[1].bar(
        np.array(range(1, energies.shape[0] + 1)),
        100 * np.std(energies, axis=1) / np.mean(energies, axis=1),
        fill=False,
        label="Standard Deviation",
    )
    axs[1].bar(
        np.array(range(1, energies.shape[0] + 1)),
        100 * quantile_deviation / np.mean(energies, axis=1),
        fill=True,
        color="black",
        label="Qartile Deviation",
    )
    axs[1].legend()
    axs[1].set_ylabel("Percent (%)")
    axs[1].set_title("Measurement Deviation in Percent")
    median_avg = np.mean(np.median(energies, axis=1))  # baseline
    means = np.mean(energies, axis=1)
    baseline_diff = means - median_avg
    baseline_diff_percentages = (baseline_diff / median_avg) * 100
    axs[2].bar(
        range(1, baseline_diff_percentages.shape[0] + 1),
        baseline_diff_percentages,
        fill=False,
    )
    axs[2].set_ylabel("Percent (%)")
    axs[2].set_title("Deviation Samplerate mean to mean of all medians")
    energy_per_sample_median = np.median(np.array(energy_per_sample), axis=1)
    # for samplerate in samplerates[:-10]:
    #     print(f"\\rotatebox{{90}}{{\\SI{{{samplerate}}}{{S/\\second}}}} &")
    for eps in energy_per_sample_median[:-10]:
        print(f"\\rotatebox{{90}}{{\\SI{{{eps:.2e}}}{{\\joule/S}}}} &")
    print()
    for eps in energy_per_sample_median[-10:]:
        print(f"\\rotatebox{{90}}{{\\SI{{{eps:.2e}}}{{\\joule/S}}}} &")
    stats = [
        f", e_per_s: {y:.2e}, median duration: {np.median(z):.2f}, sample count {np.median(a)}"
        for y, z, a in zip(energy_per_sample_median, durations, samplecounts)
    ]
    for stat in stats:
        print(stat)
    for ax in axs:
        ax.tick_params("x", rotation=90)
        ax.xaxis.grid(True)
        ax.yaxis.grid(True)
        ax.set_xticks(
            np.arange(1, len(samplerates) + 1),
            labels=[str(x) for x in samplerates],
        )
        ax.set_xlabel("Samplerate (S/s)")
    fig.tight_layout()
    fig.savefig("figure.pdf", format="pdf")
    plt.show()
