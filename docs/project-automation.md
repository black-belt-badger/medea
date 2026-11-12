# Automatic Project #7 assignment

GitHub Free supports one built-in *Auto-add to project* workflow per
project; the shared board uses that for Kirke. This repository adds
Medea-labeled issues using GitHub Actions instead.

## One-time GitHub secret setup

Create a **dedicated fine-grained personal access token** in GitHub:

- Resource owner: `black-belt-badger`
- Repository access: only `medea`
- Repository permissions: `Issues: Read-only` and `Pull requests: Read-only`
- Organization permission: `Projects: Read and write`

Store it as an **Actions repository secret** named `ADD_TO_PROJECT_PAT`
under Medea > Settings > Secrets and variables > Actions (New repository secret).
Configure an expiration and rotate the token when necessary.

The workflow cannot operate before the secret has been set. The token must
belong to an account with permission to edit organization Project #7.
Check the secret **name**, never its value, using the repository's Actions
secrets settings.

Do **not** put the token in source code, issue comments or logs. Do not
reuse your normal, broadly scoped GitHub CLI token.

## Behavior

The workflow `.github/workflows/add-to-project.yml` runs when an issue
is opened, reopened or labeled. It adds issues with the `Medea` label to
[Black Belt Badger Project #7](https://github.com/orgs/black-belt-badger/projects/7).

Existing tickets #3 and #7–#15 were added to the board separately; the
workflow handles future tickets. To verify once the secret is configured,
add the `Medea` label to a new issue and check its Actions run and Project
sidebar entry.

The default `GITHUB_TOKEN` is not sufficient to update organization
Projects. If the dedicated token expires or is revoked, replace the secret.
