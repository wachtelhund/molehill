set -e
rm -rf dist && mkdir -p dist
npx esbuild src/main.ts --bundle --minify --target=es2020 --outfile=dist/main.js
cp index.html dist/
