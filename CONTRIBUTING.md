# Contributing to stream-watcher

First off, thank you for considering contributing to stream-watcher. It's people like you that will make stream-watcher a great tool.

The following is a set of guidelines for contributing to stream-watcher. These are mostly guidelines, not rules. Use your best judgment, and feel free to propose changes to this document in a pull request. Be aware that there are many ways to contribute, from writing tutorials or blog posts, improving the documentation, submitting bug reports and feature requests or writing code. Please ensure you adhere to our [Code of Conduct](CODE_OF_CONDUCT.md)

## How can I contribute

### Bug Reports

* Check if your bug has already been reported in the issue tracker.
* If you can't find the bug or feature, please open a new issue.
* When you are creating a bug report, please include as many details as possible, and fill out the [required template](.github/TEMPLATES/issue_template.md).

### Suggesting Enhancements

* When you are creating an enhancement suggestion, please include as many details as possible.
* Fill in the [template](.github/TEMPLATES/feature_template.md), including the steps that you imagine you would take if the feature you're requesting existed.

### Pull Requests

* To avoid duplication of effort, it may be a good idea to check out the list of pending pull requests.
* Please follow these steps to have your contribution considered by the maintainer:
  1. Follow the [styleguide](#style-guide).
  2. After you submit your pull request, verify that all status checks are passing.

## Style Guide

### Git Commit Messages

* Use the present tense ("Add feature" not "Added feature").
* Use the imperative mood ("Move cursor to..." not "Moves cursor to...").
* Limit the first line to 72 characters or less.
* Reference issues and pull requests liberally after the first line.
* Be descriptive and informative in the commit message.

### Coding Convention

#### Tests

If at all practicable, all new code should be tested to some extent. If writing a test is hard, we need to prioritize making it easier, and potentially block features if that is the case.

#### Commit size

If a commit is too long to review, it should be split up into smaller pieces with tests. The exact length varies but passing the 1000 line mark should give significant thought to splitting.

When a commit is split, each commit should still be a valid state (tests passing, etc). If you must, you can gate in-progress changes with a flag or similar until the final commit.

##### TODO/FIXME/XXX Comments

    * TODO: acknowledgement that something is acceptably-for-now incomplete, especially if the scope of fixing it is high or unknown
    * FIXME: this should be fixed before the feature or major change that it's a part of is considered "ready"
    * XXX: this is bad, you are writing down that it's ugly but leaving it as-is as a better way wasn't found

### Issue Labels

#### Issue Types

| Label name | Description |
| --- | --- |
| `enhancement` | Feature requests. |
| `bug` | Confirmed bugs or reports that are very likely to be bugs. |
| `question` | Questions more than bug reports or feature requests (e.g. how do I do X). |
| `feedback` | General feedback more than bug reports or feature requests. |
| `help-wanted` | The stream-watcher developers would appreciate help from the community in resolving these issues. |
| `beginner` | Less complex issues which would be good first issues to work on for users who want to contribute to stream-watcher. |
| `more-information-needed` | More information needs to be collected about these problems or feature requests (e.g. steps to reproduce). |
| `needs-reproduction` | Likely bugs, but haven't been reliably reproduced. |
| `blocked` | Issues blocked on other issues. |
| `duplicate` | Issues which are duplicates of other issues, i.e. they have been reported before. |
| `wontfix` | The stream-watcher developers has decided not to fix these issues for now, either because they're working as intended or for some other reason. |
| `invalid` | Issues which aren't valid (e.g. user errors). |

#### Topic Categories

| Label name | Description |
| --- | --- |
| `windows` | Related to stream-watcher running on Windows. |
| `linux` | Related to stream-watcher running on Linux. |
| `mac` | Related to stream-watcher running on macOS. |
| `build-error` | Related to problems with building stream-watcher from source. |
| `documentation` | Related to any type of documentation. |
| `performance` | Related to performance. |
| `security` | Related to security. |
| `ui` | Related to visual design. |
| `uncaught-exception` | Issues about uncaught exceptions. |
| `crash` | Reports of stream-watcher completely crashing. |
| `encoding` | Related to character encoding. |
| `git` | Related to Git functionality (e.g. problems with gitignore files or with showing the correct file status). |

#### Pull Request Labels

| Label name | Description
| --- | --- |
| `work-in-progress` | Pull requests which are still being worked on, more changes will follow. |
| `needs-review` | Pull requests which need code review, and approval from stream-watcher developers. |
| `under-review` | Pull requests being reviewed by stream-watcher developers. |
| `requires-changes` | Pull requests which need to be updated based on review comments and then reviewed again. |
| `needs-testing` | Pull requests which need manual testing. |
