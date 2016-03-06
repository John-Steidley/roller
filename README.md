Roller is a tool for running multiple linters and caching the results.

[![Clippy Linting Result](http://clippy.bashy.io/github/john-steidley/roller/master/badge.svg)](http://clippy.bashy.io/github/john-steidley/roller/master/log)

To use Roller, add a `.roller_config.json` file to your project. Here's an example:

    {
      "filetypes": {
        "js": [
          {
            "name": "eslint",
            "command": "node_modules/.bin/eslint",
            "args": ["--config", "eslint.json", "--color"]
          },
          {
            "name": "lintspaces",
            "command": "lintspaces",
            "args": ["-t"]
          }
        ]
      },
      "global_ignore": [
        ".git",
        "node_modules",
        "vendor",
        "bower_components"
      ]
    }

Then just run `roller`.
