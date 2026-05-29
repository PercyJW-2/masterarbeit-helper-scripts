#!/usr/bin/bash

# "/home/nx/yolo/yolo11n-fp32.engine 15000 yolo/fp32"
# "/home/nx/yolo/yolo11n-fp16.engine 26000 yolo/fp16"
# "/home/nx/yolo/yolo11n-int8.engine 34000 yolo/int8"
# "/home/nx/hpc/pwgemmnet-fp32.engine 100 gemm/fp32"
# "/home/nx/hpc/pwgemmnet-fp16.engine 200 gemm/fp16"
# "/home/nx/hpc/pwgemmnet-int8.engine 400 gemm/int8"

benchmark_configs=(
  "/home/nx/llm_launch.sh ignore llm"
  "/home/nx/random_load/random_pattern.sh 100 random_pattern_yolo"
)

for current_config in "${benchmark_configs[@]}"; do
  # read -r engine_path engine_iterations run_path <<<"$current_config"
  read -r command parameter run_path <<<"$current_config"
  # uv run measurement_suite.py -c "ssh nx@10.42.0.200 /usr/src/tensorrt/bin/trtexec --loadEngine=$engine_path --iterations=$engine_iterations" \
  #   -m /mnt/6e97041d-abf4-4100-8bef-9111a0c14742/power_measurements/finishedMeasurements/samplerates/"$run_path" -r 15 -p \
  #   --picoscope_use_measured_voltages --pico_samplerate_sweep
  uv run measurement_suite.py -c "ssh nx@10.42.0.200 $command $parameter" \
    -m /mnt/6e97041d-abf4-4100-8bef-9111a0c14742/power_measurements/finishedMeasurements/samplerates/"$run_path" -r 15 -p \
    --picoscope_use_measured_voltages --pico_samplerate_sweep
  sshpass -f /home/jwachsmuth/.ssh/pass_file ssh twix mkdir /homes/jwachsmuth/power_measurements/sweep_without_filter/"$run_path"
  rsync -z -P -r --rsh="sshpass -f /home/jwachsmuth/.ssh/pass_file ssh -l jwachsmuth" /mnt/6e97041d-abf4-4100-8bef-9111a0c14742/power_measurements/finishedMeasurements/samplerates/"$run_path" \
    twix:/homes/jwachsmuth/power_measurements/sweep_without_filter/"$run_path"
  rm /mnt/6e97041d-abf4-4100-8bef-9111a0c14742/power_measurements/finishedMeasurements/samplerates/"$run_path"/*/*/*.npy
done
