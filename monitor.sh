#!/bin/bash

LOG_FILE="$HOME/system_monitor.log"
INTERVAL=60  # seconds between measurements

# Create log file with header if it doesn't exist
if [ ! -f "$LOG_FILE" ]; then
    echo "Timestamp,CPU_Usage(%),RAM_Used(MB),RAM_Total(MB),RAM_Usage(%),CPU_Temp(°C)" > "$LOG_FILE"
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
    
    # Write to log file (CSV format)
    echo "$TIMESTAMP,$CPU_USAGE,$RAM_USED,$RAM_TOTAL,$RAM_PERCENT,$CPU_TEMP" >> "$LOG_FILE"
    
    sleep $INTERVAL
done
