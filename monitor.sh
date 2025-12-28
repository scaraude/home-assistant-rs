#!/bin/bash

LOG_FILE="$HOME/system_monitor.log"
PROCESS_LOG_FILE="$HOME/process_monitor.log"
TOP_CPU_CONSUMERS_LOG="$HOME/top_cpu_consumers.log"
TOP_RAM_CONSUMERS_LOG="$HOME/top_ram_consumers.log"
INTERVAL=60  # seconds between measurements

# Processes to monitor
MONITORED_PROCESSES=("monitor.sh" "home-automation-rs" "zigbee2mqtt" "mosquitto")

# Create system log file with header if it doesn't exist
if [ ! -f "$LOG_FILE" ]; then
    echo "Timestamp,CPU_Usage(%),RAM_Used(MB),RAM_Total(MB),RAM_Usage(%),CPU_Temp(°C)" > "$LOG_FILE"
fi

# Create process log file with header if it doesn't exist
if [ ! -f "$PROCESS_LOG_FILE" ]; then
    echo "Timestamp,Process,PID,CPU(%),RAM(MB),Status" > "$PROCESS_LOG_FILE"
fi

# Create top CPU consumers log file with header if it doesn't exist
if [ ! -f "$TOP_CPU_CONSUMERS_LOG" ]; then
    echo "Timestamp,Rank,Process,PID,CPU(%),RAM(MB)" > "$TOP_CPU_CONSUMERS_LOG"
fi

# Create top RAM consumers log file with header if it doesn't exist
if [ ! -f "$TOP_RAM_CONSUMERS_LOG" ]; then
    echo "Timestamp,Rank,Process,PID,CPU(%),RAM(MB)" > "$TOP_RAM_CONSUMERS_LOG"
fi

while true; do
    # Get timestamp
    TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')

    # Get CPU usage (average over 1 second)
    CPU_USAGE=$(top -bn2 -d 1 | grep "Cpu(s)" | tail -n1 | awk '{print $2}' | cut -d'%' -f1)

    # Get RAM info
    RAM_INFO=$(free -m | grep Mem)
    RAM_TOTAL=$(echo $RAM_INFO | awk '{print $2}')
    RAM_USED=$(echo $RAM_INFO | awk '{print $3}')
    RAM_PERCENT=$(awk "BEGIN {printf \"%.1f\", ($RAM_USED/$RAM_TOTAL)*100}")

    # Get CPU temperature
    CPU_TEMP=$(vcgencmd measure_temp | cut -d'=' -f2 | cut -d"'" -f1)

    # Write system metrics to log file (CSV format)
    echo "$TIMESTAMP,$CPU_USAGE,$RAM_USED,$RAM_TOTAL,$RAM_PERCENT,$CPU_TEMP" >> "$LOG_FILE"

    # Monitor specific processes
    for PROCESS in "${MONITORED_PROCESSES[@]}"; do
        # Find PIDs for this process
        PIDS=$(pgrep -f "$PROCESS")

        if [ -z "$PIDS" ]; then
            # Process not running
            echo "$TIMESTAMP,$PROCESS,N/A,0.0,0.0,Not Running" >> "$PROCESS_LOG_FILE"
        else
            # Process is running, may have multiple instances
            for PID in $PIDS; do
                # Get process stats
                PS_STATS=$(ps -p $PID -o %cpu,%mem,rss --no-headers 2>/dev/null)

                if [ ! -z "$PS_STATS" ]; then
                    CPU_PROC=$(echo $PS_STATS | awk '{print $1}')
                    RAM_MB=$(awk "BEGIN {printf \"%.1f\", $(echo $PS_STATS | awk '{print $3}')/1024}")

                    echo "$TIMESTAMP,$PROCESS,$PID,$CPU_PROC,$RAM_MB,Running" >> "$PROCESS_LOG_FILE"
                fi
            done
        fi
    done

    # Get top 10 CPU consumers
    RANK=0
    ps aux --sort=-%cpu | head -n 11 | tail -n 10 | while read -r line; do
        PROC_USER=$(echo $line | awk '{print $1}')
        PROC_PID=$(echo $line | awk '{print $2}')
        PROC_CPU=$(echo $line | awk '{print $3}')
        PROC_MEM=$(echo $line | awk '{print $4}')
        PROC_CMD=$(echo $line | awk '{for(i=11;i<=NF;i++) printf $i" "; print ""}' | sed 's/ $//')

        # Calculate RAM in MB
        PROC_RAM_MB=$(awk "BEGIN {printf \"%.1f\", ($RAM_TOTAL * $PROC_MEM / 100)}")

        # Get rank (1-10)
        RANK=$((RANK+1))

        echo "$TIMESTAMP,$RANK,$PROC_CMD,$PROC_PID,$PROC_CPU,$PROC_RAM_MB" >> "$TOP_CPU_CONSUMERS_LOG"
    done

    # Get top 10 RAM consumers
    RANK=0
    ps aux --sort=-%mem | head -n 11 | tail -n 10 | while read -r line; do
        PROC_USER=$(echo $line | awk '{print $1}')
        PROC_PID=$(echo $line | awk '{print $2}')
        PROC_CPU=$(echo $line | awk '{print $3}')
        PROC_MEM=$(echo $line | awk '{print $4}')
        PROC_CMD=$(echo $line | awk '{for(i=11;i<=NF;i++) printf $i" "; print ""}' | sed 's/ $//')

        # Calculate RAM in MB
        PROC_RAM_MB=$(awk "BEGIN {printf \"%.1f\", ($RAM_TOTAL * $PROC_MEM / 100)}")

        # Get rank (1-10)
        RANK=$((RANK+1))

        echo "$TIMESTAMP,$RANK,$PROC_CMD,$PROC_PID,$PROC_CPU,$PROC_RAM_MB" >> "$TOP_RAM_CONSUMERS_LOG"
    done

    sleep $INTERVAL
done
