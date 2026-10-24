# Loads the .env at the repo root, plus two helpers. Usage: . "$(dirname "$0")/env.sh"
# Variables already set in the environment win over .env, so a run can be redirected:
#   KERNEL_DOWNSTREAM_DIR=/tmp/k scripts/build-kernel
HANOIP_ROOT=$(cd "$(dirname "$0")/.." && pwd)
if [ ! -f "$HANOIP_ROOT/.env" ]; then
	echo "Missing $HANOIP_ROOT/.env (copy .env.example and fill it in)." >&2
	exit 1
fi
eval "$(sed -n -E 's/^([A-Za-z_][A-Za-z0-9_]*)=(.*)$/export \1="${\1:-\2}"/p' "$HANOIP_ROOT/.env")"

# run CMD...: execute, or only print it when DRY_RUN is set.
run() {
	if [ -n "${DRY_RUN:-}" ]; then printf '+ %s\n' "$*"; else "$@"; fi
}

# confirm "question": ask y/N; ASSUME_YES=1 (or DRY_RUN) skips the question.
confirm() {
	[ -n "${ASSUME_YES:-}${DRY_RUN:-}" ] && return 0
	printf '%s [y/N] ' "$1"
	read -r answer
	[ "$answer" = y ] || [ "$answer" = Y ]
}
