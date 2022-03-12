class Color:

    def GREY(text):
        return '\033[90m' + text + '\033[0m'

    def BLUE(text):
        return '\033[34m' + text + '\033[0m'

    def GREEN(text):
        return '\033[32m' + text + '\033[0m'

    def YELLOW(text):
        return '\033[33m' + text + '\033[0m'

    def RED(text):
        return '\033[31m' + text + '\033[0m'

    def PURPLE(text):
        return '\033[35m' + text + '\033[0m'

    def DARKCYAN(text):
        return '\033[36m' + text + '\033[0m'

    def BOLD(text):
        return '\033[1m' + text + '\033[0m'

    def UNDERLINE(text):
        return '\033[4m' + text + '\033[0m'

#print(color.GREEN("hello"))
