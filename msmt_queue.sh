#!/usr/bin/bash

# "trtexec /home/nx/yolo/yolo11n-fp32.engine 15000 yolo/fp32"
# "trtexec /home/nx/yolo/yolo11n-fp16.engine 26000 yolo/fp16"
# "trtexec /home/nx/yolo/yolo11n-int8.engine 34000 yolo/int8"
# "trtexec /home/nx/hpc/pwgemmnet-fp32.engine 100 gemm/fp32"
# "trtexec /home/nx/hpc/pwgemmnet-fp16.engine 200 gemm/fp16"
# "trtexec /home/nx/hpc/pwgemmnet-int8.engine 400 gemm/int8"
# "trtexec /home/nx/llm_launch.sh ignore llm"

trt_exec_cmd() {
  echo "/usr/src/tensorrt/bin/trtexec --loadEngine=$1 --iterations=$2"
}

cmd=""
run_path=""
parse_run_cmd() {
  read -r type arg0 arg1 run_path_l <<<"$1"
  run_path="$run_path_l"
  if [ "$type" == "trtexec" ]; then
    cmd="$(trt_exec_cmd "$arg0" "$arg1")"
  elif [ "$type" == "other" ]; then
    cmd="$arg0 $arg1"
  elif [ "$type" == "otherNoarg" ]; then
    cmd="$arg0"
    run_path="$arg1"
  fi
}

benchmark_configs=(
  #  "other /home/nx/random_load/random_pattern.sh 100 random_pattern_yolo"
  #  "trtexec /home/nx/hpc/pwgemmnet-fp16.engine 200 gemm/fp16"
  #  "trtexec /home/nx/hpc/pwgemmnet-int8.engine 400 gemm/int8"
  # "other /home/nx/timed_engine_execution.sh /home/nx/hpc/pwgemmnet-int8.engine gemm/int8"
  "other /home/nx/timed_engine_execution.sh /home/nx/yolo/yolo11n-fp32.engine yolo/fp32"
  "other /home/nx/timed_engine_execution.sh /home/nx/yolo/yolo11n-fp16.engine yolo/fp16"
  "other /home/nx/timed_engine_execution.sh /home/nx/yolo/yolo11n-int8.engine yolo/int8"
  "otherNoarg /home/nx/random_load/random_pattern.sh random_pattern_yolo_durations_adjusted"
)

for current_config in "${benchmark_configs[@]}"; do
  if [ "$current_config" == "preload_llm" ]; then
    ssh nx@10.42.0.200 /home/nx/llm_launch.sh >/dev/null
    continue
  fi
  parse_run_cmd "$current_config"
  # uv run measurement_suite.py -c "ssh nx@10.42.0.200 /usr/src/tensorrt/bin/trtexec --loadEngine=$engine_path --iterations=$engine_iterations" \
  #   -m /mnt/6e97041d-abf4-4100-8bef-9111a0c14742/power_measurements/finishedMeasurements/samplerates/"$run_path" -r 15 -p \
  #   --picoscope_use_measured_voltages --pico_samplerate_sweep
  uv run measurement_suite.py -c "ssh nx@10.42.0.200 $cmd" \
    -m /mnt/6e97041d-abf4-4100-8bef-9111a0c14742/power_measurements/finishedMeasurements/durations/"$run_path" -r 15 -p \
    --picoscope_samplerate 2000 --duration_sweep --measurement_environment jetson -s -f -j
  sshpass -f /home/jwachsmuth/.ssh/pass_file ssh twix mkdir /homes/jwachsmuth/power_measurements/durations_without_filter/"$run_path"
  rsync -z -P -r --rsh="sshpass -f /home/jwachsmuth/.ssh/pass_file ssh -l jwachsmuth" /mnt/6e97041d-abf4-4100-8bef-9111a0c14742/power_measurements/finishedMeasurements/durations/"$run_path" \
    twix:/homes/jwachsmuth/power_measurements/durations_without_filter/"$run_path"/
  # rm /mnt/6e97041d-abf4-4100-8bef-9111a0c14742/power_measurements/finishedMeasurements/durations/"$run_path"/*/*/*.npy
done

msmt_to_repeat=(
  #  "85 100 trtexec /home/nx/hpc/pwgemmnet-fp16.engine 200 gemm/fp16/85Sps"
  #  "400 100 trtexec /home/nx/hpc/pwgemmnet-fp16.engine 200 gemm/fp16/400Sps"
  #  "1200 100 trtexec /home/nx/hpc/pwgemmnet-fp16.engine 200 gemm/fp16/1200Sps"
  #  "5500 100 trtexec /home/nx/hpc/pwgemmnet-fp16.engine 200 gemm/fp16/5500Sps"
  #  "27000 100 trtexec /home/nx/hpc/pwgemmnet-fp16.engine 200 gemm/fp16/27000Sps"
  #  "45000 100 trtexec /home/nx/hpc/pwgemmnet-fp16.engine 200 gemm/fp16/45000Sps"
  #  "160000 100 trtexec /home/nx/hpc/pwgemmnet-fp16.engine 200 gemm/fp16/160000Sps"
  #  "400000 100 trtexec /home/nx/hpc/pwgemmnet-fp16.engine 200 gemm/fp16/400000Sps"
  #  "625000 100 trtexec /home/nx/hpc/pwgemmnet-fp16.engine 200 gemm/fp16/625000Sps"
  #  "800000 100 trtexec /home/nx/hpc/pwgemmnet-fp16.engine 200 gemm/fp16/800000Sps"
  #  "2500000 100 trtexec /home/nx/hpc/pwgemmnet-fp16.engine 200 gemm/fp16/2500000Sps"
  #  "625000 101 trtexec /home/nx/hpc/pwgemmnet-int8.engine 400 gemm/int8/625000Sps"
  #  "5000000 101 trtexec /home/nx/hpc/pwgemmnet-int8.engine 400 gemm/int8/5000000Sps"
  #  "400 99 trtexec /home/nx/yolo/yolo11n-fp32.engine 15000 yolo/fp32/400Sps"
  #"preload_llm"
  #"85 136 other /home/nx/llm_launch.sh 0 llm/85Sps"
  #"400 136 other /home/nx/llm_launch.sh 0 llm/400Sps"
  #"5500 136 other /home/nx/llm_launch.sh 0 llm/5500Sps"
  #"9400 136 other /home/nx/llm_launch.sh 0 llm/9400Sps"
  #"27000 136 other /home/nx/llm_launch.sh 0 llm/27000Sps"
  #"45000 136 other /home/nx/llm_launch.sh 0 llm/45000Sps"
  #"80000 136 other /home/nx/llm_launch.sh 0 llm/80000Sps"
)

for repeated in "${msmt_to_repeat[@]}"; do
  if [ "$repeated" == "preload_llm" ]; then
    ssh nx@10.42.0.200 /home/nx/llm_launch.sh
    continue
  fi
  read -r sample_rate duration run_args <<<"$repeated"
  parse_run_cmd "$run_args"
  uv run measurement_suite.py -c "ssh nx@10.42.0.200 $cmd" \
    -m /mnt/6e97041d-abf4-4100-8bef-9111a0c14742/power_measurements/finishedMeasurements/samplerates/"$run_path" -r 15 -p \
    --picoscope_use_measured_voltages --picoscope_samplerate "$sample_rate" --duration "$duration"
  rsync -z -P -r --sh="sshpass -f /home/jwachsmuth/.ssh/pass_file ssh -l jwachsmuth" /mnt/6e97041d-abf4-4100-8bef-9111a0c14742/power_measurements/finishedMeasurements/samplerates/"$run_path"/ \
    twix:/homes/jwachsmuth/power_measurements/sweep_without_filter/llm/"$run_path"/
  rm /mnt/6e97041d-abf4-4100-8bef-9111a0c14742/power_measurements/finishedMeasurements/samplerates/"$run_path"/*/*.npy
done
