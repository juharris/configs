#!/bin/bash

# Common bash for my computers.

# For performance analysis.
# _start_time=`date +%s.%N`

THIS_SCRIPT_URL='https://raw.githubusercontent.com/juharris/configs/refs/heads/main/terminal/common.sh'

download_file_from_url() {
	url="$1"
	path="$2"
	curl --location "${url}" --output "${path}"
}

# Check if we're using Zsh
_terminal_type='bash'
if [ -n "$ZSH_VERSION" ]; then
	_terminal_type='zsh'
fi

# Check for Mac or msys with switch or cases when statements within each case
case "$OSTYPE" in
	darwin*)
		is_mac='true'
		is_win='false';;
	msys*)
		is_mac='false'
		is_win='true';;
esac

install_justin_common_tools() {
	# Meant to just be run once on a new machine for temporary installation.
	# The following is meant to standalone so that it can be copied and run on a new machine.
	if [ ! -f ~/.common.sh ]; then
		curl --location 'https://raw.githubusercontent.com/juharris/configs/refs/heads/main/terminal/common.sh' --output ~/.common.sh
		
		profile_path="$HOME/.bashrc"
		if [ ! -f "${profile_path}" ]; then
			profile_path="$HOME/.profile"
		fi
		mkdir --parents ~/workspace

		# TODO Make work for Zsh.
		# BASH_SOURCE=/Users/jus/.common.sh emulate ksh -c '. "$BASH_SOURCE"'
		if [ "${_terminal_type}" == "bash" ]; then
			echo -e "\nsource ~/.common.sh" >> "${profile_path}"
			source ~/.common.sh

			update_inputrc
		fi
	fi
}

# Update this script when installing without using Git.
update_justin_common_tools() {
	download_file_from_url ${THIS_SCRIPT_URL} ~/.common.sh.new
	echo "Difference:"
	diff --context=4 ~/.common.sh ~/.common.sh.new || true
	mv ~/.common.sh.new ~/.common.sh

	# TODO Make work for Zsh.
	# BASH_SOURCE=/Users/jus/.common.sh emulate ksh -c '. "$BASH_SOURCE"'
	if [ "${_terminal_type}" == "bash" ]; then
		echo -e "\nsource ~/.common.sh" >> "${profile_path}"
		source ~/.common.sh

		update_inputrc
	fi
}

# Update `.inputrc`.
update_inputrc() {
	local backup_path=~/.inputrc.bak.`date +%s`
	if [ -f ~/.inputrc ]; then
		mv ~/.inputrc ${backup_path}
	fi
	download_file_from_url 'https://raw.githubusercontent.com/juharris/configs/refs/heads/main/terminal/inputrc' ~/.inputrc
	if [ -f ${backup_path} ]; then
		diff --context=4 ${backup_path} ~/.inputrc || true
	fi
	bind -f ~/.inputrc
}

if [ "${_terminal_type}" == "zsh" ]; then
	# Press arrow keys up and down to search history by prefix.
	bindkey "^[[A" history-beginning-search-backward
	bindkey "^[[B" history-beginning-search-forward

	# Use Home and End to go to the beginning and end of the line.
	bindkey "^[[H" beginning-of-line
	bindkey "^[[F" end-of-line

	export CLICOLOR=1
	export LSCOLORS='ExGxBxDxCxEgEdxbxgxcxd'
else
	export LS_COLORS=${LS_COLORS}"di=1;33:"
fi

# Java
setup_java() {
	export _JAVA_OPTIONS="-Xmx4g"

	if [ -z ${JAVA_HOME+x} ]; then
		if [ -d /usr/lib/jvm/java-8-oracle ]; then
			export JAVA_HOME=/usr/lib/jvm/java-8-oracle
		fi
	fi
}

# setup_java()

export HISTSIZE=100000
export HISTFILESIZE=100000

export POETRY_HOME="$HOME/.poetry"
export PATH="${HOME}/bin:${POETRY_HOME}/bin:${PATH}"

# .NET
# dotnet_format_staged() {
# 	# sln_path=
# 	dotnet format src/*.sln --severity info --no-restore --include `git diff --name-only --staged`
# }

# Common aliases
alias l='ls -CF'
alias la='ls -A'
alias ll='ls -alFh'
# ls `--color=always` is the default.
# alias ls='ls --color=auto'

# NPM
alias npx='npx --no-install'

# grep
if [ "$is_mac" == "true" ]; then
	# On Mac, `grep` doesn't have `--color=auto`.
	alias grep='grep --color'
else
	# Try to see if setting the color is available.
	# grep --help | grep colo > /dev/null && alias grep='grep --color=auto'
	# `--color=always` should help with showing colors when piping to `less -R`.
	grep --help | grep colo > /dev/null  2>&1 && alias grep='grep --color=always'
fi

alias ..='cd ..'
alias ...='cd ../..'

# Git
alias gc='git checkout'
alias gs='git switch'
# Checkout and pull the default branch.
# Would need to do `git remote set-head origin --auto` if a repo changes the default.
# From https://stackoverflow.com/questions/28666357/git-how-to-get-default-branch#comment97435365_44750379
alias gcm='git fetch origin "`basename $(git symbolic-ref refs/remotes/origin/HEAD)`" && git checkout "`basename $(git symbolic-ref refs/remotes/origin/HEAD)`" && git merge origin/"`git rev-parse --abbrev-ref HEAD`"'
# Pull the default branch.
alias gpm='git pull --no-edit origin "`basename $(git symbolic-ref refs/remotes/origin/HEAD)`"'
# To put the branch name at the begining of a commit message:
# alias gac='git add --all && git commit -am "`git rev-parse --abbrev-ref HEAD`:'
alias gac='git add --all && git commit --message'
alias gp='git push origin "`git rev-parse --abbrev-ref HEAD`"'
alias gpb='git pull --no-edit origin "`git rev-parse --abbrev-ref HEAD`"'
alias s='git status'
alias d='git diff'
alias gds='git diff --staged'

gacp() {
	message=$1
	git add --all && git commit --message "${message}"
	git push origin "`git rev-parse --abbrev-ref HEAD`"
}

git_conditionally_commit() {
	message=$1
	if [ ! -z "$(git status --short)" ]; then
		git add --all && git commit --message "${message}"
	fi
}

export -f git_conditionally_commit > /dev/null 2>&1

gacs() {
	message=$1
	git submodule foreach --recursive "bash -c git_conditionally_commit \"${message}\""
	git_conditionally_commit "${message}"
}

git_delete_merged_branches() {
	local branches_to_delete=$(git branch --merged | grep -E -v "^(\*|`git symbolic-ref --short HEAD`| *master$| *main$)")
	if [ "${branches_to_delete}" != "" ]; then
		git branch -d ${branches_to_delete}
	fi
}

git_prune_local_branches() {
	local branches_to_delete=$(git branch | grep -E -v "^(\*|`git symbolic-ref --short HEAD`| *master$| *main$)")
	if [ "${branches_to_delete}" != "" ]; then
		git branch -D ${branches_to_delete}
	fi
}

alias gitgraph='git log --graph --all --decorate'

# Git checkout and update submodules.
gcsub() {
	git checkout $@ && \
	gpb && \
	git submodule update --init --recursive
}

git_add_nw() {
	git diff -U0 -w --no-color | git apply --cached --ignore-whitespace --unidiff-zero -
}

git_find_change_commits() {
	pattern=$1
	filename=${2-Config}
	for commithash in $(git grep "${pattern}" $(git rev-list --all) -- ${filename} | awk -F : '{print $1}'); do git log -n 1 $commithash ; done
}

git_tag_push() {
	tag="$1"
	git tag ${tag} && git push origin ${tag}
}

if [[ "$is_win" == "true" ]]; then
	# Show branch in PS1
	paths=(/c/Program\ Files/Git/mingw64/share/git/completion/git-prompt.sh /c/Program\ Files/Git/mingw64/share/git/completion/git-completion.bash /c/Program\ Files\ \(x86\)/Git/etc/git-prompt.sh /c/Program\ Files\ \(x86\)/Git/etc/git-completion.bash /e/Program\ Files\ \(x86\)/Git/etc/git-prompt.sh /e/Program\ Files\ \(x86\)/Git/etc/git-completion.bash)
	for path in "${paths[@]}"
	do
		[ -f "$path" ] && source "$path"
	done

	# Make
	mingw_make_path_candidates=(/c/MinGW/x86_64-5.3.0-release-posix-seh-rt_v4-rev0/mingw64/bin /mingw/x86_64-5.2.0-posix-seh-rt_v4-rev0/mingw64/bin /mingw/x86_64-5.1.0-posix-seh-rt_v4-rev0/mingw64/bin /mingw/bin/)
	for mingw_make_path_candidate in "${mingw_make_path_candidates[@]}"
	do
		if [ -f "$mingw_make_path_candidate/mingw32-make.exe" ]; then
			num_processors_for_make='1'
			if [ -z "$NUMBER_OF_PROCESSORS" ]; then
				NUMBER_OF_PROCESSORS=`grep -c processor /proc/cpuinfo`
			fi
			num_processors_for_make=`expr "$NUMBER_OF_PROCESSORS" - 1`
			make() {
				# Override the PATH because Msys adds some stuff at the top that doesn't work in MinGW.
				PATH=$mingw_make_path_candidate:$PATH mingw32-make.exe -j${num_processors_for_make} $@
			}
			export -f make
			break
		fi
	done

	# OneDrive
	OneDrive_path_candidates=("$OneDrive" /c/Users/juharri.NORTHAMERICA/OneDrive\ -\ Microsoft /c/Users/juharri/OneDrive\ -\ Microsoft)
	for OneDrive_path_candidate in "${OneDrive_path_candidates[@]}"
	do
		if [ "${OneDrive_path_candidate}" == "" ]; then
			continue
		fi
		if [ -d "/mnt$OneDrive_path_candidate" ]; then
			OneDrive_path="/mnt${OneDrive_path_candidate}"
			break
		fi
		if [ -d "${OneDrive_path_candidate}" ]; then
			OneDrive_path="${OneDrive_path_candidate}"
			if type 'cygpath' > /dev/null 2>&1; then
				OneDrive_path=`cygpath $OneDrive_path`
			fi
			break
		fi
	done
	unset OneDrive_path_candidate OneDrive_path_candidates
fi

# Set up SSH Agent Forwarding. Use `ssh -A` when connecting.
ssh_agent_setup() {
	if [ -z "$SSH_AUTH_SOCK" ]; then
		eval `ssh-agent -s`
		ssh-add
	fi
}

# Editor
# Might need `sudo apt-get install realpath` for `code` in WSL.
editors_candidates=(code nano vim vi)
for editors_candidate in "${editors_candidates[@]}"
do
	if type "$editors_candidate" > /dev/null 2>&1; then
		alias nn="$editors_candidate"
		export EDITOR="$editors_candidate"
		break
	fi
done
unset editors_candidate editors_candidates

# Downloads
downloads_candidates=($USERPROFILE/Downloads /c/Users/juharri.NORTHAMERICA/Downloads /c/Users/juharri/Downloads ~/downloads /c/Users/justi/Downloads /c/Users/Justin/Downloads /e/Downloads)
for downloads_candidate in "${downloads_candidates[@]}"
do
	if [[ -d /mnt$downloads_candidate ]]; then
		downloads=/mnt$downloads_candidate
		break
	fi
	if [[ -d $downloads_candidate ]]; then
		downloads=$downloads_candidate
		if type 'cygpath' > /dev/null 2>&1; then
			downloads=`cygpath "$downloads"`
		fi
		break
	fi
done
unset downloads_candidate downloads_candidates

# Workspace
# Prefer '~/workspace' for WSL2.
workspace_candidates=(~/src/github.com/Shopify ~/workspace $USERPROFILE/workspace /c/Users/justi/workspace /c/Users/juharri/workspace /e/Documents/workspace /c/Users/Justin/workspace /c/Users/Justin/Documents/workspace /workspace)
for workspace_candidate in "${workspace_candidates[@]}"
do
	if [ -d /mnt$workspace_candidate ]; then
		wksp=/mnt$workspace_candidate
		break
	fi
	if [ -d $workspace_candidate ]; then
		wksp=$workspace_candidate
		if type 'cygpath' > /dev/null 2>&1; then
			wksp=`cygpath "$wksp"`
		fi
		break
	fi
done
unset workspace_candidate workspace_candidates

# Python
install_poetry() {
	curl -sSL https://install.python-poetry.org | python -
	# Can also do the following in a Docker container:
	# pip install --user poetry
	# To rely on the existing Python env/Conda envs:
	# poetry config virtualenvs.create false
}

# `conda init` handles this now.
# conda_paths=(/c/dev/miniconda3/etc/profile.d/conda.sh /c/Anaconda3/etc/profile.d/conda.sh $HOME/miniconda3/etc/profile.d/conda.sh /c/dev/Anaconda3/etc/profile.d/conda.sh)
# for conda_path in "${conda_paths[@]}"
# do
# 	if [ -f ${conda_path} ]; then
# 		echo "Found ${conda_path}"
# 		. ${conda_path}
# 		break
# 	fi
# done
# unset conda_path conda_paths

# python_paths=($HOME/miniconda3/bin)
# for python_path in "${python_paths[@]}"
# do
# 	if [ -d ${python_path} ]; then
# 		export PATH="${python_path}:$PATH"
# 		break
# 	fi
# done

# Conda
# Remove Conda Env
conda_remove() {
	# From https://docs.conda.io/projects/conda/en/latest/user-guide/tasks/manage-environments.html#removing-an-environment
	conda remove --name $1 --all
}

alias w='cd $wksp'

# SSH
if [ "$is_mac" == 'false' ] && [ -f ~/.ssh/config ]; then
	# Complete for scp, ssh using host file.
	complete -o default -o nospace -W "$(grep -i -e '^host ' ~/.ssh/config | awk '{print substr($0, index($0,$2))}' ORS=' ')" ssh scp sftp
fi

# To add your id_rsa.pub public key to authorized_keys on a remote server:
# ssh-copy-id username@server

pwd_fetch_pull() {
	echo
	echo "*****************************************************"
	echo
	basename `pwd`
	git fetch --prune --tags
	git pull origin "`git rev-parse --abbrev-ref HEAD`"
	echo
	echo "*****************************************************"
	echo
}

sf() {
	sort --unique --ignore-case $1 > $1~ && mv $1~ $1
}

replace_in_files() {
	# example: replace_in_files 'original' 'replacement'
	# example: replace_in_files 'original' 'replacement' "*.java"
	if [ "$3" == "" ]; then
		# All files:
		find . -type f -exec sed -i 's/'$1'/'$2'/g' {} +
	else
		# Specific files:
		find . -name $3 | xargs sed -i 's/'$1'/'$2'/g'
	fi
}

# Probably not great on Windows.
# See https://docs.anaconda.com/miniconda/miniconda-install/
if [ "${is_win}" != "true" ]; then
	install_python() {(set -e
		# See https://docs.conda.io/en/latest/miniconda.html
		if [[ -z ${downloads+x} ]]; then
			downloads="${HOME}/downloads"
		fi
		mkdir --parents ${downloads}
		pushd ${downloads}
		url="https://repo.anaconda.com/miniconda/Miniconda3-latest-Linux-x86_64.sh"
		echo "Downloading ${url}"
		curl -O "${url}"
		popd
		# -b for batch mode
		# -f for no error if prefix exists
		# -u for update
		sha256sum "${downloads}/Miniconda3-latest-Linux-x86_64.sh"
		bash "${downloads}/Miniconda3-latest-Linux-x86_64.sh" -b -f -u

		export PATH="${HOME}/miniconda3/bin:${PATH}"
		conda update conda -n base
		. $HOME/miniconda3/etc/profile.d/conda.sh
		conda init
		conda init bash

		conda install --name base pip
		conda run --name base pip install --upgrade pip
	)}
fi

install_common_python_deps() {
	conda update conda -n base
	python -m pip install --upgrade pip
	conda_deps="cython ipython numpy pandas scikit-learn scipy"
	if [ "${is_win}" == "true" ]; then
		conda_deps+=" m2w64-toolchain mingw mkl libpython"
	fi
	conda install ${conda_deps}
	conda update ${conda_deps}
	pip install --upgrade injector requests six tqdm
}

# Set up a command to run Python 3.
# type -t python3 > /dev/null 2>&1 || {
# 	paths=(/e/Program\ Files/Anaconda/envs/python3 /c/Anaconda/envs/python3)
# 	for path in "${paths[@]}"
# 	do
# 		[ -d "$path" ] && export python3_path=${path}:${path}/Scripts:${python3_path}
# 	done

# 	if [[ ${python3_path} != "" ]]; then
# 		if [[ `python -c 'import sys;print(sys.version_info > (3,))'` == "True" ]]; then
# 			# Python 3 is the default.
# 			alias py='python'
# 			alias python3='python'
# 			alias py3='python'
# 			alias pip3='pip'
# 		else
# 			# Python 2 is the default.
# 			function python3
# 			{
# 				PATH=$python3_path:$PATH python "$@"
# 			}
# 			alias py3='python3'
# 			function pip3
# 			{
# 				PATH=$python3_path:$PATH pip "$@"
# 			}
# 			alias py='python'
# 			alias py2='python'
# 			alias pip2='pip'
# 		fi
# 		export -f python3
# 		export -f pip3
# 	else
# 		alias py='python'
# 		alias py2='python'
# 		alias pip2='pip'
# 	fi
# }

if [ "${is_mac}" != "true" ]; then
	type -t readlink > /dev/null 2>&1 || { function readlink
	{
		if [ "$1" == "-f" ]; then
			shift
		fi
		local path="$1" ; shift
		if [ -d "$path" ]
		then
			echo "$(cd "$path" ; pwd)"
		else
			local b=$(basename "$path")
			local p=$(dirname "$path")
			echo "$(cd "$p" ; pwd)/$b"
		fi
	}
	export -f readlink
	}

	type -t watch > /dev/null 2>&1 || { function loop
	{
		while true;
		do
			# FIXME Handle more complicated commands with quotes.
			eval "$@"
			sleep 2
		done
	}
	export -f loop
	}

	type -t nproc > /dev/null 2>&1 || { function nproc
	{
		if [ -z "$NUMBER_OF_PROCESSORS" ]; then
			NUMBER_OF_PROCESSORS=`grep -c processor /proc/cpuinfo`
		fi
		echo $NUMBER_OF_PROCESSORS
	}
	export -f nproc
	}
fi

# GPG
# Generate: gpg --full-generate-key
# Follow prompts. For email use email in GitHub: juharris@users.noreply.github.com. Give no comment.
# For GitHub:
# See keys: gpg --list-secret-keys --keyid-format LONG
# Export: gpg --armor --export <part after rsaXXXX/>

# git config --global commit.gpgsign true
# git config --global user.signingkey <part after rsaXXXX/>
# Might need: git config --global user.email juharris@users.noreply.github.com
# Run `git config --global -e` and then copy contents of ./gitconfig to it.

# Extend expiry:
# From: https://paul.fawkesley.com/extend-pgp-key-expiry-with-gpg/
# $ gpg --edit-key EB32A7EE7CF618BA
# key 0
# expire
# 3m
# y
# [repeat for subkeys: key 1, key 2 etc...]
# save

# Windows Linux Subsystem (WSL)
if [ -f /proc/sys/kernel/osrelease ] && grep -qi Microsoft /proc/sys/kernel/osrelease; then
	is_wsl="true"
	PS1=''

	if type -t git &> /dev/null; then
		export GPG_TTY=$(tty)
	fi
	# To make Theano work.
	# export KMP_AFFINITY=disabled

	# if type -t git &> /dev/null; then
	# 	git config --global core.editor "nano"
	# fi

	# For those of you who have trouble with arrow keys or key combinations in terminal programs started from windows, i found a solution that works: Install the Xming X11 server and apt-get a terminal program (konsole didn't work for me since dbus wasn't running, but xterm/urxvt worked out of the box). Start xming and bash, then
	# xterm -fg white -bg black
	#export DISPLAY=:0.0
	#and run your terminal emulator. In that window, all key combinations are supported without issues even over ssh.
else
	is_wsl="false"
	if [ "${is_mac}" != "true" ]; then
		PS1='\[\033]0;${HOSTNAME}\007'
	fi
fi

if [ "${is_mac}" != "true" ]; then
	# Set shell prompt
	if type -t __git_ps1 &> /dev/null; then
		export PS1+='\[\033[01;32m\]\h\[\033[00m\]:\[\033[01;33m\]`sed "s/\(\(\(\/mnt\)\?\/c\/Users\/\(Justin\|juharri\(\\.NORTHAMERICA\)\?\|justi\)\)\(\/Documents\)\?\|~\)\/workspace/w/g" <<< "\w"`/\[\033[00m\]$(__git_ps1)'$'\n\$ '
	else
		export PS1+='\[\033[01;32m\]\h\[\033[00m\]:\[\033[01;33m\]`sed "s/\(\(\(\/mnt\)\?\/c\/Users\/\(Justin\|juharri\(\\.NORTHAMERICA\)\?\|justi\)\)\(\/Documents\)\?\|~\)\/workspace/w/g" <<< "\w"`/\[\033[00m\]\n\$ '
	fi
fi

# npm
install_npm() {
	curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.1/install.sh | bash
	# Restart terminal
	# nvm install --lts v10
	# nvm alias default v10
	# nvm use default
	# npm install -g yarn
	# npm upgrade -g npm yarn
	# Restart terminal
}

# Shopify
if type dev > /dev/null 2>&1; then
	alias v='dev'
fi

# Clear space on Ubuntu
# sudo apt-get autoremove --purge
# sudo apt-get autoclean
# sudo apt-get clean
# sudo rm -rf /var/cache/apt/archives/*

# Docker Notes
# Install: https://docs.docker.com/install/linux/docker-ce/ubuntu/#install-using-the-convenience-script
# curl -fsSL https://get.docker.com -o get-docker.sh
# sudo sh get-docker.sh
# sudo usermod -aG docker `whoami`

unset _terminal_type

# _end_time=`date +%s.%N`
# duration=$(python3 -c "print(${_end_time} - ${_start_time})")
# echo "common.sh duration: ${duration}s"
