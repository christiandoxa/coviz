package main

func main() {
	serve()
	worker()
}

func serve() {
	worker()
}

func worker() {}

func ignored() {
	println("builtin")
}
