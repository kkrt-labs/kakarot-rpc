#!/bin/bash

set -e

echo "Forcibly stopping all running containers..."
docker stop $(docker ps -aq) 2>/dev/null || true
echo "All running containers stopped."

echo "Forcibly removing all containers..."
docker rm -f $(docker ps -aq) 2>/dev/null || true
echo "All containers removed."

echo "Forcibly removing all images..."
docker rmi -f $(docker images -q) 2>/dev/null || true
echo "All images removed."

echo "Forcibly removing all volumes..."
docker volume rm -f $(docker volume ls -q) 2>/dev/null || true
echo "All volumes removed."

echo "Forcibly removing all networks..."
docker network rm $(docker network ls -q) 2>/dev/null || true
echo "All networks removed."

echo "Forcibly pruning the system..."
docker system prune -a --volumes -f 2>/dev/null || true
echo "System prune complete."

echo "Docker cleanup complete."
