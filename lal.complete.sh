# lal(1) completion

_lal()
{
    local cur prev words cword
    _init_completion || return

    local -r subcommands="build clean configure export fetch help init script
                list-components remove shell stash status update upgrade verify"

    local has_sub
    for (( i=0; i < ${#words[@]}-1; i++ )); do
        if [[ ${words[i]} == @(build|clean|configure|export|script|fetch|help|init|list-components|remove|shell|stash|status|update|upgrade|verify) ]]; then
            has_sub=1
        fi
    done

    # global flags
    if [[ $prev = 'lal' && "$cur" == -* ]]; then
        COMPREPLY=( $(compgen -W '-v -h -V --version --help' -- "$cur" ) )
        return 0
    fi
    # first subcommand
    if [[ -z "$has_sub" ]]; then
        COMPREPLY=( $(compgen -W "$subcommands" -- "$cur" ) )
        return 0
    fi

    # special subcommand completions
    local special i
    for (( i=0; i < ${#words[@]}-1; i++ )); do
        if [[ ${words[i]} == @(build|remove|update|script) ]]; then
            special=${words[i]}
        fi
    done

    if [[ -n $special ]]; then
        case $special in
            build)
                # lal can get the keys from manifest.components
                local -r components=$(lal list-components)
                COMPREPLY=($(compgen -W "$components" -- "$cur"))
                ;;
            update)
                # Looking in local cache for allowed component names
                # Means this won't work first time, but will be quick
                local -r globals=$(find $HOME/.lal/cache/globals/ -maxdepth 1 -mindepth 1 -type d -printf "%f " 2> /dev/null)
                COMPREPLY=($(compgen -W "$globals" -- "$cur"))
                ;;
            remove)
                # look in INPUT here, nothing else makes sense
                local -r installed=$(find $PWD/INPUT/ -maxdepth 1 -mindepth 1 -type d -printf "%f " 2> /dev/null)
                COMPREPLY=($(compgen -W "$installed" -- "$cur"))
                ;;
            script)
                # look in INPUT here, nothing else makes sense
                local -r scripts=$(find $PWD/.lal/scripts/ -type f -printf "%f " 2> /dev/null)
                COMPREPLY=($(compgen -W "$scripts" -- "$cur"))
                ;;
        esac
    fi

    return 0
} &&
complete -F _lal lal

