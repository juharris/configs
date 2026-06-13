# For a nice PS1.

# `autoload` cannot be used in bash.
# It was needed when using `%{$fg[cyan]%}`, but seems like it's not needed with `%F{cyan}`.
# autoload -U colors && colors

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

function show_git_branch {
	get_git_branch_modifiers branch modifiers
	if [ -n "$branch" ]; then
		echo " (%F{cyan}%U$branch%u%f${modifiers})"
	fi
}

setopt PROMPT_SUBST;
# `%#` at the end of the line with show `#` for root, `%` for user.
# Used to use `%m` for the ${HOST}, but it would change the HOST which is annoying when other programs change it.
_host="$(hostname -s)"
PS1="%(?..%F{red}[%?]%f )%F{magenta}%n%f@%F{green}${_host}%f %F{yellow}%B%~%b%f\$(show_git_branch) "$'\n'"%# "
