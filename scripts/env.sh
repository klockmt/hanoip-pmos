# Loads the .env at the repo root. Usage: . "$(dirname "$0")/env.sh"
HANOIP_ROOT=$(cd "$(dirname "$0")/.." && pwd)
if [ ! -f "$HANOIP_ROOT/.env" ]; then
	echo "Missing $HANOIP_ROOT/.env (copy .env.example and fill it in)." >&2
	exit 1
fi
set -a
. "$HANOIP_ROOT/.env"
set +a
