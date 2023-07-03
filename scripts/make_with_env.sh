#!/bin/bash

if [ ! -f .env ]; then
    cp .env.example .env
    echo "Created .env file from .env.example"
else
    echo ".env file already exists"
fi

make $@
