#!/usr/bin/env bash
set -uo pipefail

PROJECT_KEY="${1:-job-service-rust}"
PROJECT_NAME="${2:-Job Service Rust}"
SONAR_TOKEN="${SONAR_TOKEN:?SONAR_TOKEN environment variable is required}"
SONAR_HOST="${SONAR_HOST:-http://localhost:9000}"
SCANNER_BIN="${SCANNER_BIN:-sonar-scanner}"

export LANG="${LANG:-C.UTF-8}"
export LC_ALL="${LC_ALL:-C.UTF-8}"

echo "=========================================="
echo " SonarQube scan"
echo "  Project:    $PROJECT_KEY"
echo "  Name:       $PROJECT_NAME"
echo "=========================================="

rm -rf .sonarqube

echo ""
echo ">> Step 1/2: Build + tests with coverage"
cargo test 2>/dev/null || echo "WARN: Tests failed"
cargo tarpaulin --workspace --timeout 300 --out Xml --output-dir coverage 2>/dev/null || echo "WARN: Coverage failed"
set -e

echo ""
echo ">> Step 2/2: SonarQube scan"
if [ -f "coverage/cobertura.xml" ]; then
  rm -rf .sonarqube
  "$SCANNER_BIN" \
    -Dsonar.host.url="$SONAR_HOST" \
    -Dsonar.token="$SONAR_TOKEN" \
    -Dsonar.projectKey="$PROJECT_KEY" \
    -Dsonar.projectName="$PROJECT_NAME" \
    -Dsonar.sources="src" \
    -Dsonar.tests="tests" \
    -Dsonar.exclusions="**/target/**,**/coverage/**" \
    -Dsonar.coverageReportPaths="coverage/cobertura.xml" || echo "WARN: scanner failed (exit $?)"
else
  echo "WARN: coverage/cobertura.xml not found"
fi

echo ""
echo "=========================================="
echo " Done. Dashboard:"
echo "  $SONAR_HOST/dashboard?id=$PROJECT_KEY"
echo "=========================================="
