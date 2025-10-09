export default Component()
  .data(() => ({
    abc: 123,
    obj: {
      fieldA: 'str',
      fieldB: true,
    }
  }))
  .methods({
    methodA() {
      return 1
    },
  })
  .init(({ method }) => {
    const methodB = method(() => {
      return 2
    })
    return {
      methodB,
    }
  })
  .register()
