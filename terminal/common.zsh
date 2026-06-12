# For a nice PS1.

# `autoload` cannot be used in bash.
# It was needed when using `%{$fg[cyan]%}`, but seems like it's not needed with `%F{cyan}`.
# autoload -U colors && colors

function show_git_branch {
local branch change_modifiers git_status has_staged_changes has_unstaged_changes
branch="$(command git symbolic-ref --quiet --short HEAD 2> /dev/null)"
if [ -z "${branch}" ]; then
    branch="$(command git rev-parse --short HEAD 2> /dev/null)"
fi

if [ -n "${branch}" ]; then
    command git diff-index --quiet --cached HEAD -- 2> /dev/null
    git_status=$?
    case "${git_status}" in
        1)
            has_staged_changes=1
            ;;
        128)
            if ! command git diff --no-ext-diff --cached --quiet -- 2> /dev/null; then
                has_staged_changes=1
            fi
            ;;
    esac

    if ! command git diff --no-ext-diff --quiet -- 2> /dev/null; then
        has_unstaged_changes=1
    elif command git ls-files --others --exclude-standard --directory --no-empty-directory -- ':/' 2> /dev/null | read -r; then
        has_unstaged_changes=1
    fi

    change_modifiers=""
    if [ -n "${has_staged_changes}" ]; then
        change_modifiers="+"
    fi

    if [ -n "${has_unstaged_changes}" ]; then
        change_modifiers="${change_modifiers}*"
    fi

    echo " (%F{cyan}%U$branch%u%f${change_modifiers})"
fi
}

setopt PROMPT_SUBST;
# `%#` at the end of the line with show `#` for root, `%` for user.
# Used to use `%m` for the ${HOST}, but it would change the HOST which is annoying when other programs change it.
_host="$(hostname -s)"
PS1="%(?..%F{red}[%?]%f )%F{magenta}%n%f@%F{green}${_host}%f %F{yellow}%B%~%b%f\$(show_git_branch) "$'\n'"%# "

# Case insensitive completion
autoload -Uz compinit && compinit
zstyle ':completion:*' matcher-list 'm:{a-zA-Z}={A-Za-z}'

# Press arrow keys up and down to search history by prefix.
bindkey "^[[A" history-beginning-search-backward
bindkey "^[[B" history-beginning-search-forward

# Use Home and End to go to the beginning and end of the line.
bindkey "^[[H" beginning-of-line
bindkey "^[[F" end-of-line

export CLICOLOR=1
export LSCOLORS='ExGxBxDxCxEgEdxbxgxcxd'

_current_dir=`dirname "$(realpath $0)"`
BASH_SOURCE="${_current_dir}/common.sh" emulate ksh -c '. "$BASH_SOURCE"'
