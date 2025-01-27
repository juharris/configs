# For a nice PS1.

# `autoload` cannot be used in bash.
# It was needed when using `%{$fg[cyan]%}`, but seems like it's not needed with `%F{cyan}`.
# autoload -U colors && colors

function parse_git_branch {
branch=`git rev-parse --abbrev-ref HEAD 2> /dev/null`
if [ -n "${branch}" ]; then
    echo "(%F{cyan}%U$branch%u%f)"
fi
}

setopt PROMPT_SUBST;
# `%#`: `#` for root, `%` for user
PS1="%F{magenta}%n%f@%F{green}%m%f %F{yellow}%B%~%b%f \$(parse_git_branch) "$'\n'"%# "

_current_dir=`dirname "$(realpath $0)"`
BASH_SOURCE="${_current_dir}/common.sh" emulate ksh -c '. "$BASH_SOURCE"'
