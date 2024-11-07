#!/bin/bash

# Variables
SERVICE_NAME="good_morning"
BINARY_PATH="/path/to/your/binary/program"
USER=$(whoami)

# Create systemd service file
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"
TIMER_FILE="/etc/systemd/system/${SERVICE_NAME}.timer"

echo "Creating $SERVICE_FILE..."

sudo bash -c "cat > $SERVICE_FILE" << EOF
[Unit]
Description=Good Morning Service

[Service]
Type=simple
ExecStart=$BINARY_PATH
User=$USER

[Install]
WantedBy=multi-user.target
EOF

echo "Creating $TIMER_FILE..."

sudo bash -c "cat > $TIMER_FILE" << EOF
[Unit]
Description=Runs Good Morning Service daily at noon

[Timer]
OnCalendar=*-*-* 12:00:00
Persistent=true

[Install]
WantedBy=timers.target
EOF

# Reload systemd manager configuration
echo "Reloading systemctl daemon..."
sudo systemctl daemon-reload

# Enable and start the timer
echo "Enabling and starting the timer..."
sudo systemctl enable ${SERVICE_NAME}.timer
sudo systemctl start ${SERVICE_NAME}.timer

echo "Setup complete. The program will run daily at 12:00."

