#!/usr/bin/env bash
set -e

cd "$(dirname "${BASH_SOURCE[0]}")"
cd tauri
npm install
exec npm run dev
