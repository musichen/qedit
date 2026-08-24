# Qedit documentation website

This is a dependency-light static site for GitHub Pages. It uses Alpine.js from jsDelivr for navigation, responsive menus, fuzzy search, and small interactions. The page is deployed by `.github/workflows/website.yml` to:

`https://musichen.github.io/qedit/`

The content cards link to the canonical Markdown documentation in this repository. To preview locally, serve this folder with any static file server, for example:

```sh
python3 -m http.server 8080 --directory website
```

Then open <http://localhost:8080>.
