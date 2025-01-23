# For a nice PS1.
# Cannot be done in bash.
autoload -U colors && colors

function parse_git_branch {
branch=`git rev-parse --abbrev-ref HEAD 2> /dev/null`
if [ -n "${branch}" ]; then
    echo "(%{$fg[cyan]%}$branch%{$reset_color%})"
fi
}

setopt PROMPT_SUBST;
PS1="%{$fg[magenta]%}%n%{$reset_color%}@%{$fg[green]%}%m %{$fg[yellow]%}%~ %{$reset_color%}\$(parse_git_branch) "$'\n'"%% "

_current_dir=`dirname "$(realpath $0)"`
BASH_SOURCE="${_current_dir}/common.sh" emulate ksh -c '. "$BASH_SOURCE"'
