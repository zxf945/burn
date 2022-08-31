use crate::module::{ADModule, Module, State, StateNamed};
use crate::optim::Optimizer;
use crate::tensor::{back, Data, Gradients, Tensor};

#[derive(Debug)]
pub struct Param<T> {
    value: T,
}

impl<T> std::ops::Deref for Param<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> Param<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

impl<const D: usize, B: back::Backend> Param<Tensor<B, D>> {
    pub fn num_params(&self) -> usize {
        self.value.shape().num_elements()
    }

    pub fn update_params<O: Optimizer<Backend = B>>(&mut self, grads: &Gradients, optim: &mut O)
    where
        B: back::ad::Backend,
    {
        optim.update(&mut self.value, grads);
    }

    pub fn devices(&self) -> Vec<B::Device> {
        vec![self.value.device()]
    }

    pub fn to_device(&mut self, device: B::Device) {
        self.value = self.value.to_device(device);
    }

    pub fn state(&self) -> State<B> {
        State::Data(self.value.to_data().serialize())
    }

    pub fn load(&mut self, state: &State<B>) {
        match state {
            State::Data(data) => {
                self.value = Tensor::from_data_device(Data::from(data), self.value.device());
            }
            _ => {}
        }
    }

    pub fn inner(&self) -> Param<Tensor<B::InnerBackend, D>>
    where
        B: back::ad::Backend,
    {
        Param::new(self.value.inner())
    }
}

impl<const D: usize, B: back::Backend> Param<Option<Tensor<B, D>>> {
    pub fn num_params(&self) -> usize {
        if let Some(value) = &self.value {
            return value.shape().num_elements();
        }

        0
    }

    pub fn update_params<O: Optimizer<Backend = B>>(&mut self, grads: &Gradients, optim: &mut O)
    where
        B: back::ad::Backend,
    {
        if let Some(value) = &mut self.value {
            optim.update(value, grads);
        }
    }

    pub fn devices(&self) -> Vec<B::Device> {
        if let Some(value) = &self.value {
            return vec![value.device()];
        }

        vec![]
    }

    pub fn to_device(&mut self, device: B::Device) {
        if let Some(value) = &self.value {
            self.value = Some(value.to_device(device));
        }
    }

    pub fn state(&self) -> State<B> {
        if let Some(value) = &self.value {
            return State::Data(value.to_data().serialize());
        }

        State::StateNamed(StateNamed::new())
    }

    pub fn load(&mut self, state: &State<B>) {
        let data = match state {
            State::Data(data) => data,
            _ => return,
        };

        if let Some(value) = &self.value {
            self.value = Some(Tensor::from_data_device(Data::from(data), value.device()));
        }
    }

    pub fn inner(&self) -> Param<Option<Tensor<B::InnerBackend, D>>>
    where
        B: back::ad::Backend,
    {
        match &self.value {
            Some(tensor) => Param::new(Some(tensor.inner())),
            None => Param::new(None),
        }
    }
}

impl<M: Module> Param<M> {
    pub fn num_params(&self) -> usize {
        self.value.num_params()
    }

    pub fn update_params<O: Optimizer<Backend = M::Backend>>(
        &mut self,
        grads: &Gradients,
        optim: &mut O,
    ) where
        M::Backend: back::ad::Backend,
    {
        self.value.update_params(grads, optim);
    }

    pub fn devices(&self) -> Vec<<M::Backend as back::Backend>::Device> {
        self.value.devices()
    }

    pub fn to_device(&mut self, device: <M::Backend as back::Backend>::Device) {
        self.value.to_device(device)
    }

    pub fn state(&self) -> State<M::Backend> {
        self.value.state()
    }

    pub fn load(&mut self, state: &State<M::Backend>) {
        self.value.load(state)
    }

    pub fn inner(&self) -> Param<M::InnerModule>
    where
        M: ADModule,
        M::Backend: back::ad::Backend,
    {
        Param::new(self.value.inner())
    }
}

impl<M: Module> Param<Vec<M>> {
    pub fn num_params(&self) -> usize {
        let mut num_params = 0;
        for module in self.value.iter() {
            num_params += module.num_params();
        }

        num_params
    }

    pub fn update_params<O: Optimizer<Backend = M::Backend>>(
        &mut self,
        grads: &Gradients,
        optim: &mut O,
    ) where
        M::Backend: back::ad::Backend,
    {
        for module in self.value.iter_mut() {
            module.update_params(grads, optim);
        }
    }

    pub fn devices(&self) -> Vec<<M::Backend as back::Backend>::Device> {
        let mut devices = Vec::new();
        for module in self.value.iter() {
            devices.append(&mut module.devices());
        }
        devices
    }

    pub fn to_device(&mut self, device: <M::Backend as back::Backend>::Device) {
        for module in self.value.iter_mut() {
            module.to_device(device);
        }
    }

    pub fn state(&self) -> State<M::Backend> {
        let mut state = StateNamed::new();

        for (i, module) in self.value.iter().enumerate() {
            state.register_state(format!("mod-{}", i).as_str(), module.state());
        }

        State::StateNamed(state)
    }

    pub fn load(&mut self, state: &State<M::Backend>) {
        for (i, module) in self.value.iter_mut().enumerate() {
            module.load(state.get(format!("mod-{}", i).as_str()));
        }
    }

    pub fn inner(&self) -> Param<Vec<M::InnerModule>>
    where
        M: ADModule,
        M::Backend: back::ad::Backend,
    {
        Param::new(self.value.iter().map(|v| v.inner()).collect())
    }
}
