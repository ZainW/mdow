# Homebrew Tap Setup (one-time)

## Create the tap repo

```bash
gh repo create ZainW/homebrew-mdow --public \
  --description "Homebrew tap for Mdow" --add-readme
```

Clone it locally, then add `Casks/mdow.rb` with this initial content (the
`update-homebrew-cask.sh` script keeps `version` and `sha256` in sync from then on):

```ruby
cask "mdow" do
  version "1.0.0"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"

  url "https://github.com/ZainW/mdow/releases/download/v#{version}/Mdow-#{version}-arm64.dmg"
  name "Mdow"
  desc "A quiet place to read markdown"
  homepage "https://github.com/ZainW/mdow"

  livecheck do
    url :url
    strategy :github_latest
  end

  auto_updates false

  app "Mdow.app"

  zap trash: [
    "~/Library/Application Support/Mdow",
    "~/Library/Preferences/com.mdow.app.plist",
    "~/Library/Logs/Mdow",
  ]
end
```

Commit and push.

## Release flow

After cutting a desktop release with `pnpm --filter desktop run publish` and
waiting for the GitHub release assets to upload:

```bash
pnpm --filter desktop run release:homebrew
```

Users then install with:

```bash
brew tap zainw/mdow      # one-time
brew install --cask mdow  # or: brew install --cask zainw/mdow/mdow
```

And update with:

```bash
brew upgrade --cask mdow
```

## Asset naming dependency

`update-homebrew-cask.sh` expects the arm64 dmg to be named
`Mdow-<version>-arm64.dmg`. This matches electron-builder's default for the
current `apps/desktop/electron-builder.yml`. If you ever change `productName`,
`artifactName`, or remove the arm64 target, update both the script and the
cask URL pattern accordingly.
